//! Adapter from immutable file-backed originals to the optimized renderer.
use crate::image_store::ImageView;

pub(super) use qingqu_picture_renderer::Transform;
#[derive(Default)]
pub(super) struct Renderer {
    inner: qingqu_picture_renderer::Renderer,
}
impl Renderer {
    pub fn render_strided(
        &mut self,
        image: &ImageView,
        source: &[u8],
        transform: Transform,
        dest: &mut [u8],
        stride: usize,
    ) {
        self.inner.render_strided(
            &qingqu_picture_renderer::ImageView {
                source_width: image.original.width(),
                source_height: image.original.height(),
                turns: image.turns,
                opaque: image.original.opaque(),
            },
            source,
            transform,
            dest,
            stride,
        );
    }
}
