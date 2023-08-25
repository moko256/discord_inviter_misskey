use chrono::{DateTime, Duration, Utc};
use futures::Future;
use tokio::time::sleep;

/// Simple exponential retry, waiting 2 ^ (error count) minutes. (1s, 1m, 2m, 4m, 8m, 16m ...)
pub async fn simple_retry_loop_by_time<F>(
    duration_reset_retry_count: Duration,
    max_retry_duration: Duration,
    block: impl Fn() -> F,
) where
    F: Future<Output = ()>,
{
    let max_retry_duration = max_retry_duration.to_std().unwrap();

    let mut count: i64 = 0;

    loop {
        let before_start = Utc::now();

        block().await;

        if can_reset(before_start, Utc::now(), duration_reset_retry_count) {
            count = 0;
        }

        let next_sleep = to_sleep_duration(count, max_retry_duration);

        count += 1;

        log::info!(
            "Retry {}, wait {} minutes.",
            count,
            next_sleep.as_secs() / 60
        );

        sleep(next_sleep).await;
    }
}

#[inline]
fn can_reset(
    before_start: DateTime<Utc>,
    after_end: DateTime<Utc>,
    duration_reset_retry_count: Duration,
) -> bool {
    after_end > (before_start + duration_reset_retry_count)
}

#[inline]
fn to_sleep_duration(count: i64, max_retry_duration: std::time::Duration) -> std::time::Duration {
    if count > 0 {
        let exp = count - 1;
        let duration = 2_u64.pow(exp.try_into().unwrap());

        std::time::Duration::from_secs(duration * 60).min(max_retry_duration)
    } else {
        std::time::Duration::from_secs(1) // Wait a little bit though count is 0.
    }
}

#[cfg(test)]
mod test {
    use chrono::{DateTime, Duration, Utc};

    use super::can_reset;

    use super::to_sleep_duration;

    #[test]
    fn can_reset_test() {
        fn date_test_minutes(n: i64) -> DateTime<Utc> {
            DateTime::from_utc(
                chrono::NaiveDateTime::from_timestamp_millis(n * 60 * 1000).unwrap(),
                Utc,
            )
        }

        let result = can_reset(
            date_test_minutes(0),
            date_test_minutes(20),
            Duration::minutes(10),
        );
        assert!(result);

        let result = can_reset(
            date_test_minutes(0),
            date_test_minutes(5),
            Duration::minutes(10),
        );
        assert!(!result);
    }

    #[test]
    fn to_sleep_duration_test() {
        assert_eq!(
            to_sleep_duration(0, std::time::Duration::from_secs(100 * 60)),
            std::time::Duration::from_secs(1)
        );
        assert_eq!(
            to_sleep_duration(1 + 4, std::time::Duration::from_secs(100 * 60)),
            std::time::Duration::from_secs(16 * 60)
        );

        assert_eq!(
            to_sleep_duration(1 + 4, std::time::Duration::from_secs(10 * 60)),
            std::time::Duration::from_secs(10 * 60)
        );
    }
}
