use gtk4::graphene::Rect;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{gdk, glib};

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Default)]
    pub struct BatteryMeter {
        pub level: Cell<f64>,
        pub limit: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BatteryMeter {
        const NAME: &'static str = "NchargeBatteryMeter";
        type Type = super::BatteryMeter;
        type ParentType = gtk4::Widget;
    }

    impl ObjectImpl for BatteryMeter {}

    impl WidgetImpl for BatteryMeter {
        fn snapshot(&self, snapshot: &gtk4::Snapshot) {
            let width = self.obj().width() as f32;
            let height = self.obj().height() as f32;

            if width <= 0.0 || height <= 0.0 {
                return;
            }

            let track_h = 8.0f32;
            let track_y = (height - track_h) / 2.0;

            // Track background
            let track_rect = Rect::new(0.0, track_y, width, track_h);
            snapshot.append_color(&gdk::RGBA::new(0.0, 0.0, 0.05, 0.10), &track_rect);

            // Fill level (blue)
            let level_pct = (self.level.get() / 100.0).clamp(0.0, 1.0) as f32;
            let fill_w = width * level_pct;
            if fill_w > 0.0 {
                let fill_rect = Rect::new(0.0, track_y, fill_w, track_h);
                snapshot.append_color(
                    &gdk::RGBA::new(0.11, 0.44, 0.85, 1.0), // #1C71D8
                    &fill_rect,
                );
            }

            // Tick for active limit
            let limit_pct = (self.limit.get() / 100.0).clamp(0.0, 1.0) as f32;
            let tick_x = (width * limit_pct - 1.0).clamp(0.0, width - 2.0);
            let tick_rect = Rect::new(tick_x, 0.0, 2.0, height);
            snapshot.append_color(&gdk::RGBA::new(0.12, 0.12, 0.14, 1.0), &tick_rect);
        }

        fn measure(&self, _orientation: gtk4::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (18, 18, -1, -1)
        }
    }
}

glib::wrapper! {
    pub struct BatteryMeter(ObjectSubclass<imp::BatteryMeter>)
        @extends gtk4::Widget;
}

impl BatteryMeter {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_level_and_limit(&self, level: f64, limit: f64) {
        let imp = self.imp();
        imp.level.set(level);
        imp.limit.set(limit);
        self.queue_draw();
    }
}

impl Default for BatteryMeter {
    fn default() -> Self {
        Self::new()
    }
}
