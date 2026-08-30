use chrono::{DateTime, Duration, Utc};

pub async fn wait_past_cutoff(cutoff: DateTime<Utc>) {
    let next_second = cutoff + Duration::seconds(1);
    let now = Utc::now();
    if next_second > now
        && let Ok(wait) = (next_second - now).to_std()
    {
        tokio::time::sleep(wait).await;
    }
}
