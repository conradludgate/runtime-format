# runtime-format

Formatting, but processed at runtime.

```rust
use runtime_format::{FormatArgs, FormatKey, FormatKeyError};
use core::fmt;

impl FormatKey for DateTime {
    fn fmt(&self, key: &str, f: &mut fmt::Formatter<'_>) -> Result<(), FormatKeyError> {
        use core::fmt::Write;
        match key {
            "year"    => write!(f, "{}", self.year()).map_err(FormatKeyError::Fmt),
            "month"   => write!(f, "{}", self.short_month_name()).map_err(FormatKeyError::Fmt),
            "day"     => write!(f, "{}", self.day()).map_err(FormatKeyError::Fmt),
            "hours"   => write!(f, "{}", self.hours()).map_err(FormatKeyError::Fmt),
            "minutes" => write!(f, "{}", self.minutes()).map_err(FormatKeyError::Fmt),
            "seconds" => write!(f, "{}", self.seconds()).map_err(FormatKeyError::Fmt),
            _ => Err(FormatKeyError::UnknownKey),
        }
    }
}
let now = DateTime::now();
let fmt = "{month} {day} {year} {hours}:{minutes}:{seconds}";
let args = FormatArgs::new(fmt, &now);

// Outputs "Jan 25 2023 16:27:53"
println!("{args}");
```
