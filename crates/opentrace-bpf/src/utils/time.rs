// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.

use std::fmt::Display;

const NANOS_PER_MILLI: u64 = 1_000_000;
const NANOS_PER_SECOND: u64 = 1_000_000_000;
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;

/// Duration value stored in nanoseconds.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TimeAsNanosecond(pub u64);

impl Display for TimeAsNanosecond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let total_seconds = self.0 / NANOS_PER_SECOND;
        let hours = total_seconds / SECONDS_PER_HOUR;
        let minutes = total_seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
        let seconds = total_seconds % SECONDS_PER_MINUTE;
        let millis = self.0 % NANOS_PER_SECOND / NANOS_PER_MILLI;

        if millis == 0 {
            write!(f, "{hours:02}:{minutes:02}:{seconds:02}")
        } else {
            write!(f, "{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
        }
    }
}

pub(crate) struct Nanoseconds(pub(crate) u64);
impl Display for Nanoseconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 >= 1_000_000_000 {
            write!(f, "{}s", self.0 / 1_000_000_000)
        } else if self.0 >= 1_000_000 {
            write!(f, "{}ms", self.0 / 1_000_000)
        } else if self.0 >= 1_000 {
            write!(f, "{}us", self.0 / 1_000)
        } else {
            write!(f, "{}ns", self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TimeAsNanosecond;

    #[test]
    fn formats_zero() {
        assert_eq!(TimeAsNanosecond(0).to_string(), "00:00:00");
    }

    #[test]
    fn formats_seconds() {
        assert_eq!(TimeAsNanosecond(1_000_000_000).to_string(), "00:00:01");
    }

    #[test]
    fn formats_milliseconds() {
        assert_eq!(TimeAsNanosecond(1_234_567_890).to_string(), "00:00:01.234");
    }

    #[test]
    fn formats_hours_minutes_seconds() {
        assert_eq!(TimeAsNanosecond(3_723_000_000_000).to_string(), "01:02:03");
    }
}
