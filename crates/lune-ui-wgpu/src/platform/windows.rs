/// Windows: DX12 with DirectComposition presentation, so the window's glass keeps its alpha.
pub(crate) fn configure(desc: &mut wgpu::InstanceDescriptor) {
    desc.backends = wgpu::Backends::DX12;
    desc.backend_options.dx12.presentation_system = wgpu::Dx12SwapchainKind::DxgiFromVisual;
}
