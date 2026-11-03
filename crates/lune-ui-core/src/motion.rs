//! Motion: the clock type the animation maths runs on. The easings, tweens, springs and
//! transitions arrive with the motion core; this module holds the time type they share.

/// Time since the app's origin, as the app supplies it in `RawInput`. The toolkit never reads a
/// clock itself.
pub type Time = std::time::Duration;
