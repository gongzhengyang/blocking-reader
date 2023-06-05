use std::fmt::Display;
use std::io::SeekFrom;
use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache;
use tokio::fs::{self, File};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::OnceCell;

static CACHE_TASK_SEEK: OnceCell<Cache<String, u64>> = OnceCell::const_new();

#[async_trait]
pub trait FileReadExt: Display {
    async fn get_cache_handle() -> &'static Cache<String, u64> {
        CACHE_TASK_SEEK
            .get_or_init(|| async { Cache::new(u16::MAX as u64) })
            .await
    }

    /// a line must include all patterns, and will get true
    /// if patterns is empty, then will get true always
    fn is_line_match_all_patterns(line: &str, patterns: &Vec<&str>) -> bool {
        for &pattern in patterns {
            if !line.contains(pattern) {
                return false;
            }
        }
        true
    }

    async fn blocking_read(
        &self,
        patterns: &Vec<&str>,
        matched_lines: &mut Vec<String>,
    ) -> anyhow::Result<()> {
        let filepath = format!("{self}");
        let create_time = fs::metadata(&filepath).await?.created()?;
        let cache_key = format!("{filepath}.{create_time:?}");
        let cache = Self::get_cache_handle().await;
        let last_offset = cache.get(&cache_key).unwrap_or(0);

        tracing::debug!("begin read {filepath} created at {create_time:?} at offset {last_offset}");
        let mut buffer_reader = BufReader::new(File::open(filepath).await?);
        let _ = buffer_reader.seek(SeekFrom::Start(last_offset)).await;
        let pin_buffer_reader = Pin::new(&mut buffer_reader);

        let mut lines = pin_buffer_reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if Self::is_line_match_all_patterns(&line, patterns) {
                matched_lines.push(line);
            }
        }
        let _ = buffer_reader.seek(SeekFrom::End(0)).await;
        let seek = buffer_reader.stream_position().await?;
        tracing::debug!("cache:{cache_key} with offset: {seek}");
        cache.insert(cache_key, seek).await;
        Ok(())
    }

    async fn blocking_read_with_time_limit(
        &self,
        patterns: &Vec<&str>,
        time_limit: Duration,
    ) -> anyhow::Result<Vec<String>> {
        let mut matched_lines = vec![];
        let _ = tokio::time::timeout(
            time_limit,
            Self::blocking_read(self, patterns, &mut matched_lines),
        )
        .await;
        Ok(matched_lines)
    }
}

impl FileReadExt for String {}

impl FileReadExt for &str {}
