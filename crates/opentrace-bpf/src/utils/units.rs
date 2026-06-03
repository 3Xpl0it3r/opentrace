// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
//
pub mod bytes {
    use std::fmt::Display;

    pub struct Bytes(pub u32);

    impl Display for Bytes {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            if self.0 >= 1024 * 1024 {
                write!(f, "{}M", self.0 / (1024 * 1024))
            } else if self.0 >= 1024 {
                write!(f, "{}k", self.0 / 1024)
            } else {
                write!(f, "{}B", self.0)
            }
        }
    }
}

pub mod time {

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

    pub struct Nanoseconds(pub u64);
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
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::bytes::Bytes;
    use super::time::{Nanoseconds, TimeAsNanosecond};

    #[rstest]
    #[case(0, "0B")]
    #[case(1023, "1023B")]
    #[case(1024, "1k")]
    #[case(1536, "1k")]
    #[case(1024 * 1024, "1M")]
    #[case(2 * 1024 * 1024 + 512, "2M")]
    fn formats_bytes_with_binary_units(#[case] input: u32, #[case] expected: &str) {
        assert_eq!(Bytes(input).to_string(), expected);
    }

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

    #[rstest]
    #[case(999, "999ns")]
    #[case(1_000, "1us")]
    #[case(1_999, "1us")]
    #[case(1_000_000, "1ms")]
    #[case(1_999_999, "1ms")]
    #[case(1_000_000_000, "1s")]
    fn formats_duration_units(#[case] input: u64, #[case] expected: &str) {
        assert_eq!(Nanoseconds(input).to_string(), expected);
    }
}
