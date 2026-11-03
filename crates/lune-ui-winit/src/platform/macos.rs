use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::WindowAttributes;

pub(crate) fn glass(attrs: WindowAttributes) -> WindowAttributes {
    attrs
        .with_titlebar_transparent(true)
        .with_fullsize_content_view(true)
        .with_title_hidden(true)
}
