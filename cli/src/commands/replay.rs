use crate::api::{ChronicleClient, Event};
use crate::error::{ChronicleError, Result};
use crate::output::OutputManager;
use crate::search::{SearchQueryBuilder, parse_filters, validate_limit};
use clap::Args;
use std::time::Duration;
use tokio::time::{sleep, Instant};

#[derive(Args, Debug)]
pub struct ReplayArgs {
    /// Time range for replay (e.g., "2024-01-01T10:00:00..10:30:00")
    #[arg(short, long)]
    pub time: String,

    /// Search query to filter events during replay
    #[arg(short, long)]
    pub query: Option<String>,

    /// Additional filters (key=value,key2=value2)
    #[arg(long)]
    pub filters: Option<String>,

    /// Replay speed multiplier (1.0 = real-time, 2.0 = 2x speed, 0.5 = half speed)
    #[arg(long, default_value = "1.0")]
    pub speed: f64,

    /// Maximum number of events to replay
    #[arg(short, long, default_value = "1000")]
    pub limit: usize,

    /// Output file for recording replay session
    #[arg(short, long)]
    pub output: Option<String>,

    /// Follow mode (continue replaying new events)
    #[arg(long)]
    pub follow: bool,

    /// Show only specific event types
    #[arg(long)]
    pub event_types: Option<String>,

    /// Pause replay at specific events (comma-separated event IDs)
    #[arg(long)]
    pub pause_at: Option<String>,

    /// Interactive mode (allow pausing/resuming)
    #[arg(short, long)]
    pub interactive: bool,

    /// Timeout for replay operation in seconds
    #[arg(long, default_value = "3600")]
    pub timeout: u64,

    /// Show detailed event information
    #[arg(long)]
    pub detailed: bool,

    /// Colorize output based on event types
    #[arg(long)]
    pub colorize: bool,
}

pub async fn run(args: ReplayArgs, client: ChronicleClient, output: OutputManager) -> Result<()> {
    // Validate arguments
    validate_limit(args.limit)?;
    
    if args.speed <= 0.0 || args.speed > 100.0 {
        return Err(ChronicleError::InvalidQuery(
            "Speed must be between 0.0 and 100.0".to_string(),
        ));
    }

    // Build search query
    let search_query = build_replay_query(&args)?;

    // Set up client with timeout
    let client = client.with_timeout(Duration::from_secs(args.timeout));

    // Execute search to get events
    let spinner = output.create_spinner("Loading events for replay...");
    let results = client.search(&search_query).await?;
    spinner.finish_with_message("✓ Events loaded");

    if results.events.is_empty() {
        output.print_warning("No events found for the specified time range")?;
        return Ok(());
    }

    output.print_info(&format!(
        "Replaying {} events from {} to {}",
        results.events.len(),
        results.events[0].timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
        results.events[results.events.len() - 1].timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    ))?;

    // Start replay
    let mut replay_session = ReplaySession::new(args, output);
    replay_session.start(results.events).await?;

    Ok(())
}

fn build_replay_query(args: &ReplayArgs) -> Result<crate::api::SearchQuery> {
    let query_str = args.query.as_deref().unwrap_or("*");
    let mut query_builder = SearchQueryBuilder::new(query_str)
        .with_time_str(&args.time)?
        .with_limit(args.limit);

    if let Some(filters_str) = &args.filters {
        let filters = parse_filters(filters_str)?;
        for (key, value) in filters {
            query_builder = query_builder.with_filter(&key, &value);
        }
    }

    // Add event type filter if specified
    if let Some(event_types) = &args.event_types {
        let types: Vec<&str> = event_types.split(',').collect();
        for event_type in types {
            query_builder = query_builder.with_filter("event_type", event_type.trim());
        }
    }

    Ok(query_builder.build())
}

struct ReplaySession {
    args: ReplayArgs,
    output: OutputManager,
    pause_at_events: Vec<String>,
    paused: bool,
    start_time: Instant,
    output_file: Option<std::fs::File>,
}

impl ReplaySession {
    fn new(args: ReplayArgs, output: OutputManager) -> Self {
        let pause_at_events = if let Some(pause_at) = &args.pause_at {
            pause_at.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            Vec::new()
        };

        Self {
            args,
            output,
            pause_at_events,
            paused: false,
            start_time: Instant::now(),
            output_file: None,
        }
    }

    async fn start(&mut self, events: Vec<Event>) -> Result<()> {
        // Open output file if specified
        if let Some(output_path) = &self.args.output {
            use std::io::Write;
            let mut file = std::fs::File::create(output_path)?;
            writeln!(file, "# Chronicle Replay Session")?;
            writeln!(file, "# Started at: {}", chrono::Utc::now())?;
            writeln!(file, "# Speed: {}x", self.args.speed)?;
            writeln!(file, "# Events: {}", events.len())?;
            writeln!(file)?;
            self.output_file = Some(file);
        }

        // Calculate time differences for proper pacing
        let mut time_diffs = Vec::new();
        for i in 1..events.len() {
            let diff = events[i].timestamp - events[i - 1].timestamp;
            time_diffs.push(diff.num_milliseconds() as f64);
        }

        // Start replay
        self.output.print_info("Starting replay (press 'q' to quit, 'p' to pause/resume in interactive mode)...")?;
        
        for (i, event) in events.iter().enumerate() {
            // Check for pause conditions
            if self.pause_at_events.contains(&event.id) {
                self.paused = true;
                self.output.print_warning(&format!("Paused at event: {}", event.id))?;
            }

            // Handle interactive mode
            if self.args.interactive {
                self.handle_interactive_input().await?;
            }

            // Wait if paused
            while self.paused {
                sleep(Duration::from_millis(100)).await;
                if self.args.interactive {
                    self.handle_interactive_input().await?;
                }
            }

            // Display event
            self.display_event(event, i + 1, events.len()).await?;

            // Log to file if specified
            if let Some(ref mut file) = self.output_file {
                use std::io::Write;
                writeln!(file, "{}: {}", event.timestamp.to_rfc3339(), event.id)?;
            }

            // Wait for next event (respecting speed multiplier)
            if i < time_diffs.len() {
                let wait_time = (time_diffs[i] / self.args.speed).max(1.0);
                sleep(Duration::from_millis(wait_time as u64)).await;
            }
        }

        // Follow mode - continue monitoring for new events
        if self.args.follow {
            self.output.print_info("Entering follow mode...")?;
            self.follow_mode().await?;
        }

        let total_time = self.start_time.elapsed();
        self.output.print_success(&format!(
            "Replay completed in {:.2} seconds",
            total_time.as_secs_f64()
        ))?;

        Ok(())
    }

    async fn display_event(&self, event: &Event, index: usize, total: usize) -> Result<()> {
        let timestamp = event.timestamp.format("%H:%M:%S%.3f");
        let progress = format!("[{:>4}/{:>4}]", index, total);
        
        if self.args.detailed {
            if self.args.colorize {
                use console::style;
                println!("{} {} {} [{}] {}",
                    style(progress).dim(),
                    style(timestamp).blue(),
                    style(&event.event_type).cyan(),
                    style(&event.id).green(),
                    serde_json::to_string(&event.data)?
                );
            } else {
                println!("{} {} {} [{}] {}",
                    progress,
                    timestamp,
                    event.event_type,
                    event.id,
                    serde_json::to_string(&event.data)?
                );
            }
        } else {
            if self.args.colorize {
                use console::style;
                println!("{} {} {} {}",
                    style(progress).dim(),
                    style(timestamp).blue(),
                    style(&event.event_type).cyan(),
                    crate::utils::truncate_string(&serde_json::to_string(&event.data)?, 80)
                );
            } else {
                println!("{} {} {} {}",
                    progress,
                    timestamp,
                    event.event_type,
                    crate::utils::truncate_string(&serde_json::to_string(&event.data)?, 80)
                );
            }
        }

        Ok(())
    }

    async fn handle_interactive_input(&mut self) -> Result<()> {
        // In a real implementation, this would handle non-blocking input
        // For now, we'll just provide a simplified version
        Ok(())
    }

    async fn follow_mode(&mut self) -> Result<()> {
        // In follow mode, we would periodically query for new events
        // and replay them as they come in
        self.output.print_info("Follow mode not yet implemented")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;
    use chrono::Utc;
    use serde_json::json;

    #[test]
    fn test_build_replay_query() {
        let args = ReplayArgs {
            time: "2024-01-01T10:00:00..2024-01-01T11:00:00".to_string(),
            query: Some("test".to_string()),
            filters: Some("type=error".to_string()),
            speed: 1.0,
            limit: 100,
            output: None,
            follow: false,
            event_types: Some("error,warning".to_string()),
            pause_at: None,
            interactive: false,
            timeout: 3600,
            detailed: false,
            colorize: false,
        };

        let query = build_replay_query(&args).unwrap();
        assert_eq!(query.query, "test");
        assert_eq!(query.limit, Some(100));
        assert!(query.filters.is_some());
    }

    #[test]
    fn test_replay_session_creation() {
        let args = ReplayArgs {
            time: "2024-01-01T10:00:00..2024-01-01T11:00:00".to_string(),
            query: None,
            filters: None,
            speed: 2.0,
            limit: 100,
            output: None,
            follow: false,
            event_types: None,
            pause_at: Some("event1,event2".to_string()),
            interactive: false,
            timeout: 3600,
            detailed: false,
            colorize: false,
        };

        let output = OutputManager::new(OutputFormat::Table, false);
        let session = ReplaySession::new(args, output);
        
        assert_eq!(session.pause_at_events, vec!["event1", "event2"]);
        assert!(!session.paused);
    }

    #[tokio::test]
    async fn test_event_display() {
        let args = ReplayArgs {
            time: "2024-01-01T10:00:00..2024-01-01T11:00:00".to_string(),
            query: None,
            filters: None,
            speed: 1.0,
            limit: 100,
            output: None,
            follow: false,
            event_types: None,
            pause_at: None,
            interactive: false,
            timeout: 3600,
            detailed: false,
            colorize: false,
        };

        let output = OutputManager::new(OutputFormat::Table, false);
        let session = ReplaySession::new(args, output);
        
        let event = Event {
            id: "test-event".to_string(),
            timestamp: Utc::now(),
            event_type: "test".to_string(),
            data: json!({"key": "value"}),
            metadata: std::collections::HashMap::new(),
        };

        // This would normally display to stdout
        assert!(session.display_event(&event, 1, 10).await.is_ok());
    }
}