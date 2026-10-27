
Rust type contracts port the JavaScript argument-count and non-integral metadata checks.
Each snippet must fail compilation. Behavioral failures remain runtime tests.
Numeric string coercion is deliberately replaced by typed integer coordinates;
callers can explicitly parse strings before calling the Rust API.

<!-- contract: key_tap_missing_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.key_tap();
```

<!-- contract: unicode_tap_missing_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.unicode_tap();
```

<!-- contract: type_string_missing_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.type_string();
```

<!-- contract: type_string_delayed_missing_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.type_string_delayed();
```

<!-- contract: move_mouse_extra_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.move_mouse(0, 1, 2, 3);
```

<!-- contract: move_mouse_missing_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.move_mouse(0);
```

<!-- contract: move_mouse_string_coordinates -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.move_mouse("0", "0");
```

<!-- contract: move_mouse_smooth_extra_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.move_mouse_smooth(0, 1, 2, 3);
```

<!-- contract: move_mouse_smooth_missing_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.move_mouse_smooth(0);
```

<!-- contract: move_mouse_smooth_string_coordinates -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.move_mouse_smooth("0", "0", None);
```

<!-- contract: mouse_click_extra_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.mouse_click(Some("left"), false, "it");
```

<!-- contract: drag_mouse_missing_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.drag_mouse(0);
```

<!-- contract: drag_mouse_extra_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.drag_mouse(1, 1, Some("left"), 5);
```

<!-- contract: get_pixel_color_missing_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.get_pixel_color(0);
```

<!-- contract: get_pixel_color_extra_arguments -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
Robot.get_pixel_color(1, 2, 3);
```

<!-- contract: color_must_be_string -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
fn check(b: &Bitmap) { b.find_color(123456, SearchOptions::default()); }
```

<!-- contract: search_rectangle_x_nan -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { x: Some(f64::NAN), ..Default::default() };
```

<!-- contract: search_rectangle_y_nan -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { y: Some(f64::NAN), ..Default::default() };
```

<!-- contract: search_rectangle_width_nan -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { width: Some(f64::NAN), ..Default::default() };
```

<!-- contract: search_rectangle_height_nan -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { height: Some(f64::NAN), ..Default::default() };
```

<!-- contract: search_rectangle_x_infinity -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { x: Some(f64::INFINITY), ..Default::default() };
```

<!-- contract: search_rectangle_y_infinity -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { y: Some(f64::INFINITY), ..Default::default() };
```

<!-- contract: search_rectangle_width_infinity -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { width: Some(f64::INFINITY), ..Default::default() };
```

<!-- contract: search_rectangle_height_infinity -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { height: Some(f64::INFINITY), ..Default::default() };
```

<!-- contract: search_rectangle_x_fractional -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { x: Some(0.5), ..Default::default() };
```

<!-- contract: search_rectangle_y_fractional -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { y: Some(0.5), ..Default::default() };
```

<!-- contract: search_rectangle_width_fractional -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { width: Some(0.5), ..Default::default() };
```

<!-- contract: search_rectangle_height_fractional -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
SearchOptions { height: Some(0.5), ..Default::default() };
```

<!-- contract: metadata_nan_dimensions -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
fn check(b: &mut Bitmap) { b.width = f64::NAN; }
```

<!-- contract: metadata_infinite_dimensions -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
fn check(b: &mut Bitmap) { b.height = f64::INFINITY; }
```

<!-- contract: metadata_fractional_stride -->
```compile_fail
use rusty_robot::{Robot, Bitmap, SearchOptions};
fn check(b: &mut Bitmap) { b.byte_width = 3.5; }
```
