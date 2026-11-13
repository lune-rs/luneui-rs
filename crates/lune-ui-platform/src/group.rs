//! Window groups: several windows of one app that the OS treats as one, so they come to the front,
//! stay on top and minimise together (D316). One window is the leader; the others are members.
//!
//! On Windows the members are owned windows of the leader: build each with
//! [`WindowGroup::member_attributes`] (or [`owned_by`]) so the OS creates it with the leader as its
//! owner. The group then has one taskbar button, and the OS raises, keeps on top and minimises it
//! as one. On macOS a member docked to the leader ([`WindowGroup::set_docked`]) is a child window
//! of the leader, which AppKit moves, orders and minimises with it; the group orders the detached
//! members by hand ([`WindowGroup::raise`], [`WindowGroup::minimise`]). Elsewhere the group only
//! records what the app asks.
//!
//! A member is never minimised alone: on Windows a minimised owned window becomes a small stub at
//! the desktop's corner, apart from its group. So `WindowCommand::Minimize` and
//! `WindowCommand::SetAlwaysOnTop` from any window of a group go through the group
//! ([`WindowGroup::minimise`], [`WindowGroup::set_always_on_top`]), not to the window itself.
//!
//! An owned member on Windows does not follow the leader's moves, and a detached member never
//! does: the app moves a docked set itself (`desktop::move_windows`).

use std::marker::PhantomData;
use std::sync::Arc;

use winit::window::{Window, WindowAttributes, WindowId, WindowLevel};

use crate::platform::group as os;

/// `attrs` with `owner` as the owner of the window they build, on Windows: the window then has no
/// taskbar button of its own, stays above its owner and minimises and rises with it. Unchanged
/// elsewhere. A popup window of a group (a menu in its own window) is built this way; a group
/// member is built with [`WindowGroup::member_attributes`], which is this with the leader. An
/// owned window must never be minimised alone (see the module docs).
pub fn owned_by(attrs: WindowAttributes, owner: &Window) -> WindowAttributes {
    os::owned_by(attrs, owner)
}

/// A member window and whether it is docked to the leader.
#[derive(Debug)]
struct Member {
    window: Arc<Window>,
    docked: bool,
}

/// A leader window and its members, treated as one by the OS. See the module docs. Use it on the
/// main thread; dropping it detaches the docked members (macOS), and the members' handles go
/// before the leader's.
#[derive(Debug)]
pub struct WindowGroup {
    /// First, so the members drop before the leader (Windows destroys owned windows with their
    /// owner).
    members: Vec<Member>,
    leader: Arc<Window>,
    /// Keeps the type `!Send` and `!Sync` on every platform: the OS calls need the main thread.
    main_thread: PhantomData<*const ()>,
}

impl WindowGroup {
    /// A group of `leader` alone.
    pub fn new(leader: Arc<Window>) -> Self {
        Self {
            members: Vec::new(),
            leader,
            main_thread: PhantomData,
        }
    }

    /// `attrs` for a member window of this group: [`owned_by`] the leader.
    pub fn member_attributes(&self, attrs: WindowAttributes) -> WindowAttributes {
        owned_by(attrs, &self.leader)
    }

    /// Adds `member`, not docked. On Windows it must have been built with
    /// [`WindowGroup::member_attributes`] (its owner is the leader), or this is an `Err`. The
    /// leader itself, or a window already in the group, is an `Err` too.
    pub fn add(&mut self, member: Arc<Window>) -> Result<(), String> {
        if self.contains(member.id()) {
            return Err("the window is already in the group".into());
        }
        os::check_member(&self.leader, &member)?;
        self.members.push(Member {
            window: member,
            docked: false,
        });
        Ok(())
    }

    /// Records whether `member` is docked to the leader. On macOS a docked member becomes a child
    /// window of the leader and a detached one stops being one; setting the same value again
    /// does nothing. On Windows and elsewhere it is only recorded (the app moves docked windows
    /// itself). An id that is not a member is an `Err`.
    pub fn set_docked(&mut self, member: WindowId, docked: bool) -> Result<(), String> {
        let m = self
            .members
            .iter_mut()
            .find(|m| m.window.id() == member)
            .ok_or("the window is not a member of the group")?;
        os::set_docked(&self.leader, &m.window, docked)?;
        m.docked = docked;
        Ok(())
    }

    /// Whether `member` is a member docked to the leader.
    pub fn is_docked(&self, member: WindowId) -> bool {
        self.members
            .iter()
            .any(|m| m.window.id() == member && m.docked)
    }

    /// Brings the rest of the group up behind `front`, the group window that just came to the
    /// front. On macOS every window not attached to the leader, other than `front`, is ordered
    /// just below it (the leader carries its docked members). Elsewhere the OS does this itself
    /// and nothing happens. A window not in the group does nothing.
    pub fn raise(&self, front: WindowId) {
        let Some(front) = self.index_of(front) else {
            return;
        };
        os::raise(&self.windows(), &self.docked(), front);
    }

    /// Minimises the group: the leader is minimised, which takes the owned (Windows) or docked
    /// (macOS) members with it; on macOS the detached members are ordered out too.
    pub fn minimise(&self) {
        self.leader.set_minimized(true);
        os::hide_detached(&self.windows(), &self.docked());
    }

    /// Restores the group from [`WindowGroup::minimise`]: the leader comes back, and on macOS the
    /// detached members are ordered in front again without taking the keys.
    pub fn restore(&self) {
        self.leader.set_minimized(false);
        os::show_detached(&self.windows(), &self.docked());
    }

    /// Keeps every window of the group above other apps' windows, or not.
    pub fn set_always_on_top(&self, on: bool) {
        let level = if on {
            WindowLevel::AlwaysOnTop
        } else {
            WindowLevel::Normal
        };
        for window in self.windows() {
            window.set_window_level(level);
        }
    }

    /// The leader.
    pub fn leader(&self) -> &Arc<Window> {
        &self.leader
    }

    /// The members, in the order they were added.
    pub fn members(&self) -> impl Iterator<Item = &Arc<Window>> {
        self.members.iter().map(|m| &m.window)
    }

    /// Whether `id` is the leader or a member.
    pub fn contains(&self, id: WindowId) -> bool {
        self.index_of(id).is_some()
    }

    /// The window's index: 0 for the leader, `i + 1` for member `i`.
    fn index_of(&self, id: WindowId) -> Option<usize> {
        if self.leader.id() == id {
            return Some(0);
        }
        self.members
            .iter()
            .position(|m| m.window.id() == id)
            .map(|i| i + 1)
    }

    /// The leader, then the members.
    fn windows(&self) -> Vec<&Window> {
        std::iter::once(&*self.leader)
            .chain(self.members.iter().map(|m| &*m.window))
            .collect()
    }

    /// Whether each member is docked, in member order.
    fn docked(&self) -> Vec<bool> {
        self.members.iter().map(|m| m.docked).collect()
    }
}

impl Drop for WindowGroup {
    /// Detaches the docked members (macOS; nothing on Windows), before any handle goes.
    fn drop(&mut self) {
        for m in &self.members {
            if m.docked {
                // Nothing to do about a refusal while dropping: the windows go anyway.
                let _ = os::set_docked(&self.leader, &m.window, false);
            }
        }
    }
}

/// The windows to order just below `front` when it comes to the front, by index (0 the leader,
/// `i + 1` member `i`; `docked[i]` whether member `i` is docked): the leader, which carries its
/// docked members, unless `front` is the leader or docked to it, then every detached member
/// other than `front`, in ascending order.
#[allow(
    dead_code,
    reason = "used only by the macOS half; unit-tested here on every host"
)]
pub(crate) fn raise_order(front: usize, docked: &[bool]) -> Vec<usize> {
    let front_with_leader = front == 0 || docked.get(front - 1).copied().unwrap_or(false);
    let mut out = Vec::new();
    if !front_with_leader {
        out.push(0);
    }
    out.extend(
        docked
            .iter()
            .enumerate()
            .filter(|&(i, &d)| !d && i + 1 != front)
            .map(|(i, _)| i + 1),
    );
    out
}

/// The members a minimised leader does not take with it on macOS, by index (`i + 1` for member
/// `i`): the detached ones.
#[allow(
    dead_code,
    reason = "used only by the macOS half; unit-tested here on every host"
)]
pub(crate) fn hidden_with_leader(docked: &[bool]) -> Vec<usize> {
    docked
        .iter()
        .enumerate()
        .filter(|&(_, &d)| !d)
        .map(|(i, _)| i + 1)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rest_of_the_group_orders_below_the_front_window() {
        assert_eq!(raise_order(0, &[true, false]), vec![2]);
        assert_eq!(raise_order(2, &[true, false]), vec![0]);
        assert_eq!(raise_order(1, &[false, false]), vec![0, 2]);
        // A docked member at the front: the leader is already with it.
        assert_eq!(raise_order(1, &[true, false]), vec![2]);
        assert_eq!(raise_order(0, &[]), Vec::<usize>::new());
    }

    #[test]
    fn a_minimised_leader_hides_the_detached_members() {
        assert_eq!(hidden_with_leader(&[true, false]), vec![2]);
        assert_eq!(hidden_with_leader(&[false, true, false]), vec![1, 3]);
        assert_eq!(hidden_with_leader(&[]), Vec::<usize>::new());
    }
}
