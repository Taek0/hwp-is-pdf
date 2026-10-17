//! Page-local origins for native HWP column definitions with local line caches.
#[derive(Default)]
pub(super) struct ColumnBands {
    page: usize,
    origin: f64,
    bottom: f64,
}
impl ColumnBands {
    fn sync(&mut self, page: usize) {
        if self.page != page {
            *self = Self {
                page,
                ..Default::default()
            };
        }
    }
    pub(super) fn start(&mut self, page: usize, first_y: f64, gap: f64) -> f64 {
        self.sync(page);
        self.origin =
            (self.bottom + if self.bottom > 0. { gap.max(0.) } else { 0. } - first_y).max(0.);
        self.origin
    }
    pub(super) fn origin(&mut self, page: usize) -> f64 {
        self.sync(page);
        self.origin
    }
    pub(super) fn observe(&mut self, page: usize, bottom: f64) {
        self.sync(page);
        self.bottom = self.bottom.max(bottom);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn band_follows_deepest_column_and_resets_on_new_page() {
        let mut b = ColumnBands::default();
        b.observe(0, 80.);
        b.observe(0, 60.);
        assert_eq!(b.start(0, 0., 7.), 87.);
        assert_eq!(b.origin(0), 87.);
        b.observe(0, 100.);
        assert_eq!(b.start(0, 10., 7.), 97.);
        assert_eq!(b.origin(1), 0.);
        b.observe(1, 25.);
        assert_eq!(b.start(1, 0., 7.), 32.);
    }
}
