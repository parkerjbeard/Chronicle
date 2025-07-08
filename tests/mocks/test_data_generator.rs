use std::collections::HashMap;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde_json::{json, Value};
use chrono::{DateTime, Utc, Duration};
use anyhow::Result;

use crate::utils::TestEvent;

/// Test data generator for creating realistic test events
pub struct TestDataGenerator {
    config: DataGeneratorConfig,
    rng: StdRng,
    sequence_counters: HashMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct DataGeneratorConfig {
    pub deterministic: bool,
    pub seed: u64,
    pub event_types: Vec<String>,
    pub time_variance_ms: i64,
    pub data_size_range: (usize, usize),
    pub realistic_data: bool,
    pub include_metadata: bool,
}

impl TestDataGenerator {
    pub fn new(config: DataGeneratorConfig) -> Self {
        let rng = if config.deterministic {
            StdRng::seed_from_u64(config.seed)
        } else {
            StdRng::from_entropy()
        };

        Self {
            config,
            rng,
            sequence_counters: HashMap::new(),
        }
    }

    pub fn generate_events(&mut self, count: usize) -> Result<Vec<TestEvent>> {
        let mut events = Vec::with_capacity(count);
        
        for i in 0..count {
            let event = self.generate_single_event(i as u64)?;
            events.push(event);
        }
        
        Ok(events)
    }

    pub fn generate_single_event(&mut self, id: u64) -> Result<TestEvent> {
        let event_type = self.select_event_type();
        let timestamp = self.generate_timestamp();
        let data = self.generate_event_data(&event_type)?;
        
        // Update sequence counter for this event type
        let sequence = self.sequence_counters.entry(event_type.clone()).or_insert(0);
        *sequence += 1;
        
        let mut event = TestEvent {
            id,
            timestamp,
            event_type,
            data,
            checksum: None,
        };

        // Calculate checksum if needed
        if self.config.include_metadata {
            event.calculate_checksum();
        }

        Ok(event)
    }

    fn select_event_type(&mut self) -> String {
        if self.config.event_types.is_empty() {
            "default".to_string()
        } else {
            let index = self.rng.gen_range(0..self.config.event_types.len());
            self.config.event_types[index].clone()
        }
    }

    fn generate_timestamp(&mut self) -> DateTime<Utc> {
        let base_time = Utc::now();
        
        if self.config.time_variance_ms > 0 {
            let variance = self.rng.gen_range(-self.config.time_variance_ms..=self.config.time_variance_ms);
            base_time + Duration::milliseconds(variance)
        } else {
            base_time
        }
    }

    fn generate_event_data(&mut self, event_type: &str) -> Result<Value> {
        if self.config.realistic_data {
            self.generate_realistic_data(event_type)
        } else {
            self.generate_simple_data(event_type)
        }
    }

    fn generate_realistic_data(&mut self, event_type: &str) -> Result<Value> {
        match event_type {
            "keytap" => Ok(self.generate_keytap_data()),
            "mouse" => Ok(self.generate_mouse_data()),
            "window" => Ok(self.generate_window_data()),
            "clipboard" => Ok(self.generate_clipboard_data()),
            "filesystem" => Ok(self.generate_filesystem_data()),
            "network" => Ok(self.generate_network_data()),
            "audio" => Ok(self.generate_audio_data()),
            "screen" => Ok(self.generate_screen_data()),
            _ => Ok(self.generate_generic_data(event_type)),
        }
    }

    fn generate_keytap_data(&mut self) -> Value {
        let keys = [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
            "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
            "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
            "space", "return", "backspace", "delete", "tab", "escape",
            "up", "down", "left", "right"
        ];
        
        let modifiers = [
            vec![],
            vec!["shift"],
            vec!["cmd"],
            vec!["ctrl"],
            vec!["alt"],
            vec!["cmd", "shift"],
            vec!["ctrl", "shift"],
        ];
        
        let applications = [
            "Chrome", "Firefox", "Safari", "VSCode", "Terminal", "Finder",
            "TextEdit", "Mail", "Messages", "Slack", "Zoom", "Word"
        ];

        let key = keys[self.rng.gen_range(0..keys.len())];
        let modifier = &modifiers[self.rng.gen_range(0..modifiers.len())];
        let app = applications[self.rng.gen_range(0..applications.len())];

        json!({
            "key": key,
            "modifiers": modifier,
            "application": app,
            "window_id": self.rng.gen_range(1000..9999),
            "key_code": self.rng.gen_range(1..255),
            "press_duration_ms": self.rng.gen_range(50..200)
        })
    }

    fn generate_mouse_data(&mut self) -> Value {
        let event_types = ["click", "move", "scroll", "drag", "hover"];
        let buttons = ["left", "right", "middle"];
        
        let event_type = event_types[self.rng.gen_range(0..event_types.len())];
        let button = buttons[self.rng.gen_range(0..buttons.len())];

        json!({
            "event_type": event_type,
            "button": button,
            "x": self.rng.gen_range(0..1920),
            "y": self.rng.gen_range(0..1080),
            "delta_x": self.rng.gen_range(-100..100),
            "delta_y": self.rng.gen_range(-100..100),
            "click_count": self.rng.gen_range(1..3),
            "pressure": self.rng.gen_range(0.0..1.0)
        })
    }

    fn generate_window_data(&mut self) -> Value {
        let applications = [
            "Chrome", "Firefox", "Safari", "VSCode", "Terminal", "Finder",
            "TextEdit", "Mail", "Messages", "Slack", "Zoom", "Word"
        ];
        
        let events = ["focus", "unfocus", "minimize", "maximize", "close", "open", "resize"];
        
        let app = applications[self.rng.gen_range(0..applications.len())];
        let event = events[self.rng.gen_range(0..events.len())];

        json!({
            "event": event,
            "application": app,
            "window_id": self.rng.gen_range(1000..9999),
            "title": format!("{} - Document {}", app, self.rng.gen_range(1..100)),
            "bounds": {
                "x": self.rng.gen_range(0..500),
                "y": self.rng.gen_range(0..500),
                "width": self.rng.gen_range(400..1200),
                "height": self.rng.gen_range(300..800)
            },
            "pid": self.rng.gen_range(1000..32768)
        })
    }

    fn generate_clipboard_data(&mut self) -> Value {
        let content_types = ["text", "image", "file", "url", "rich_text"];
        let content_type = content_types[self.rng.gen_range(0..content_types.len())];
        
        let (size, sample_content) = match content_type {
            "text" => {
                let size = self.rng.gen_range(10..1000);
                (size, "Lorem ipsum dolor sit amet...".to_string())
            },
            "image" => {
                let size = self.rng.gen_range(100000..5000000);
                (size, "[IMAGE DATA]".to_string())
            },
            "file" => {
                let size = self.rng.gen_range(1000..10000000);
                (size, "/path/to/file.txt".to_string())
            },
            "url" => {
                let size = self.rng.gen_range(20..200);
                (size, "https://example.com/path".to_string())
            },
            "rich_text" => {
                let size = self.rng.gen_range(100..5000);
                (size, "<html><body>Rich content</body></html>".to_string())
            },
            _ => (0, "".to_string())
        };

        json!({
            "content_type": content_type,
            "size_bytes": size,
            "content_hash": format!("sha256:{:x}", self.rng.gen::<u64>()),
            "source_app": "System",
            "sample_content": if size < 1000 { sample_content } else { "[LARGE CONTENT]".to_string() }
        })
    }

    fn generate_filesystem_data(&mut self) -> Value {
        let operations = ["create", "modify", "delete", "move", "rename", "access"];
        let file_types = [".txt", ".pdf", ".jpg", ".png", ".mp4", ".zip", ".doc", ".xls"];
        let directories = [
            "/Users/test/Documents",
            "/Users/test/Downloads", 
            "/Users/test/Desktop",
            "/tmp",
            "/var/log",
            "/Users/test/Pictures"
        ];

        let operation = operations[self.rng.gen_range(0..operations.len())];
        let file_ext = file_types[self.rng.gen_range(0..file_types.len())];
        let dir = directories[self.rng.gen_range(0..directories.len())];
        let filename = format!("file_{}{}", self.rng.gen_range(1..1000), file_ext);

        json!({
            "operation": operation,
            "path": format!("{}/{}", dir, filename),
            "size_bytes": self.rng.gen_range(0..10000000),
            "permissions": format!("{:o}", self.rng.gen_range(644..755)),
            "file_type": file_ext,
            "inode": self.rng.gen_range(100000..999999)
        })
    }

    fn generate_network_data(&mut self) -> Value {
        let protocols = ["TCP", "UDP", "HTTP", "HTTPS", "FTP", "SSH"];
        let local_ports = [80, 443, 22, 21, 8080, 3000, 5432, 27017];
        
        let protocol = protocols[self.rng.gen_range(0..protocols.len())];
        let local_port = local_ports[self.rng.gen_range(0..local_ports.len())];
        
        json!({
            "protocol": protocol,
            "local_address": "127.0.0.1",
            "local_port": local_port,
            "remote_address": format!("192.168.1.{}", self.rng.gen_range(1..255)),
            "remote_port": self.rng.gen_range(1024..65535),
            "bytes_sent": self.rng.gen_range(0..1000000),
            "bytes_received": self.rng.gen_range(0..1000000),
            "connection_state": "ESTABLISHED",
            "process_id": self.rng.gen_range(1000..32768)
        })
    }

    fn generate_audio_data(&mut self) -> Value {
        let devices = ["Built-in Microphone", "AirPods Pro", "USB Headset", "Built-in Output"];
        let device = devices[self.rng.gen_range(0..devices.len())];

        json!({
            "device": device,
            "sample_rate": 44100,
            "channels": self.rng.gen_range(1..3),
            "bit_depth": 16,
            "duration_ms": self.rng.gen_range(100..5000),
            "volume_level": self.rng.gen_range(0.0..1.0),
            "is_input": self.rng.gen_bool(0.5),
            "format": "PCM"
        })
    }

    fn generate_screen_data(&mut self) -> Value {
        json!({
            "screenshot_hash": format!("sha256:{:x}", self.rng.gen::<u64>()),
            "screen_size": {
                "width": 1920,
                "height": 1080
            },
            "active_window": format!("Window {}", self.rng.gen_range(1..100)),
            "cursor_position": {
                "x": self.rng.gen_range(0..1920),
                "y": self.rng.gen_range(0..1080)
            },
            "compression_ratio": self.rng.gen_range(0.1..0.9),
            "file_size_bytes": self.rng.gen_range(50000..500000)
        })
    }

    fn generate_generic_data(&mut self, event_type: &str) -> Value {
        let data_size = self.rng.gen_range(self.config.data_size_range.0..=self.config.data_size_range.1);
        
        json!({
            "event_type": event_type,
            "data": "x".repeat(data_size),
            "size": data_size,
            "random_value": self.rng.gen::<u64>()
        })
    }

    fn generate_simple_data(&mut self, event_type: &str) -> Result<Value> {
        let data_size = self.rng.gen_range(self.config.data_size_range.0..=self.config.data_size_range.1);
        
        Ok(json!({
            "type": event_type,
            "data": "x".repeat(data_size),
            "sequence": self.sequence_counters.get(event_type).copied().unwrap_or(0),
            "timestamp": Utc::now().timestamp_millis()
        }))
    }

    /// Generate events for a specific time range
    pub fn generate_time_series_events(
        &mut self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        events_per_second: f64,
    ) -> Result<Vec<TestEvent>> {
        let duration = end_time.signed_duration_since(start_time);
        let total_seconds = duration.num_seconds() as f64;
        let total_events = (total_seconds * events_per_second) as usize;
        
        let mut events = Vec::with_capacity(total_events);
        
        for i in 0..total_events {
            let progress = i as f64 / total_events as f64;
            let event_time = start_time + Duration::milliseconds((progress * total_seconds * 1000.0) as i64);
            
            let event_type = self.select_event_type();
            let data = self.generate_event_data(&event_type)?;
            
            let mut event = TestEvent {
                id: i as u64,
                timestamp: event_time,
                event_type,
                data,
                checksum: None,
            };
            
            if self.config.include_metadata {
                event.calculate_checksum();
            }
            
            events.push(event);
        }
        
        Ok(events)
    }

    /// Generate events with specific patterns for testing
    pub fn generate_pattern_events(
        &mut self,
        pattern: EventPattern,
        count: usize,
    ) -> Result<Vec<TestEvent>> {
        let mut events = Vec::with_capacity(count);
        
        for i in 0..count {
            let event = match pattern {
                EventPattern::Burst => self.generate_burst_event(i)?,
                EventPattern::Periodic => self.generate_periodic_event(i)?,
                EventPattern::Random => self.generate_single_event(i as u64)?,
                EventPattern::Sequential => self.generate_sequential_event(i)?,
            };
            events.push(event);
        }
        
        Ok(events)
    }

    fn generate_burst_event(&mut self, index: usize) -> Result<TestEvent> {
        // Generate bursts of 10 events every 100 events
        let burst_size = 10;
        let burst_interval = 100;
        let in_burst = (index % burst_interval) < burst_size;
        
        let event_type = if in_burst { "burst_event" } else { "normal_event" };
        let data = json!({
            "burst": in_burst,
            "burst_index": index % burst_size,
            "overall_index": index
        });
        
        Ok(TestEvent {
            id: index as u64,
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            data,
            checksum: None,
        })
    }

    fn generate_periodic_event(&mut self, index: usize) -> Result<TestEvent> {
        let period = 50;
        let phase = index % period;
        
        let data = json!({
            "period": period,
            "phase": phase,
            "cycle": index / period,
            "value": (phase as f64 / period as f64 * 2.0 * std::f64::consts::PI).sin()
        });
        
        Ok(TestEvent {
            id: index as u64,
            timestamp: Utc::now(),
            event_type: "periodic_event".to_string(),
            data,
            checksum: None,
        })
    }

    fn generate_sequential_event(&mut self, index: usize) -> Result<TestEvent> {
        let data = json!({
            "sequence": index,
            "is_even": index % 2 == 0,
            "fibonacci": self.fibonacci(index % 20), // Limit to prevent overflow
            "data": format!("sequential_data_{}", index)
        });
        
        Ok(TestEvent {
            id: index as u64,
            timestamp: Utc::now(),
            event_type: "sequential_event".to_string(),
            data,
            checksum: None,
        })
    }

    fn fibonacci(&self, n: usize) -> u64 {
        match n {
            0 => 0,
            1 => 1,
            _ => {
                let mut a = 0u64;
                let mut b = 1u64;
                for _ in 2..=n {
                    let temp = a.saturating_add(b);
                    a = b;
                    b = temp;
                }
                b
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum EventPattern {
    Burst,
    Periodic,
    Random,
    Sequential,
}

impl Default for DataGeneratorConfig {
    fn default() -> Self {
        Self {
            deterministic: true,
            seed: 42,
            event_types: vec![
                "keytap".to_string(),
                "mouse".to_string(),
                "window".to_string(),
                "clipboard".to_string(),
            ],
            time_variance_ms: 1000,
            data_size_range: (100, 1000),
            realistic_data: true,
            include_metadata: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_generator_creation() {
        let config = DataGeneratorConfig::default();
        let generator = TestDataGenerator::new(config);
        assert_eq!(generator.sequence_counters.len(), 0);
    }

    #[test]
    fn test_generate_events() {
        let config = DataGeneratorConfig::default();
        let mut generator = TestDataGenerator::new(config);
        
        let events = generator.generate_events(10).unwrap();
        assert_eq!(events.len(), 10);
        
        // Check that events have unique IDs
        let mut ids: Vec<u64> = events.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 10);
    }

    #[test]
    fn test_deterministic_generation() {
        let config = DataGeneratorConfig {
            deterministic: true,
            seed: 123,
            ..Default::default()
        };
        
        let mut generator1 = TestDataGenerator::new(config.clone());
        let mut generator2 = TestDataGenerator::new(config);
        
        let events1 = generator1.generate_events(5).unwrap();
        let events2 = generator2.generate_events(5).unwrap();
        
        // Events should be identical with same seed
        for (e1, e2) in events1.iter().zip(events2.iter()) {
            assert_eq!(e1.event_type, e2.event_type);
            // Note: timestamps will be different, but data structure should be similar
        }
    }

    #[test]
    fn test_pattern_generation() {
        let config = DataGeneratorConfig::default();
        let mut generator = TestDataGenerator::new(config);
        
        let events = generator.generate_pattern_events(EventPattern::Sequential, 5).unwrap();
        assert_eq!(events.len(), 5);
        
        // Check sequential pattern
        for (i, event) in events.iter().enumerate() {
            assert_eq!(event.id, i as u64);
            assert_eq!(event.event_type, "sequential_event");
        }
    }

    #[test]
    fn test_realistic_data_generation() {
        let config = DataGeneratorConfig {
            realistic_data: true,
            event_types: vec!["keytap".to_string()],
            ..Default::default()
        };
        
        let mut generator = TestDataGenerator::new(config);
        let event = generator.generate_single_event(1).unwrap();
        
        assert_eq!(event.event_type, "keytap");
        assert!(event.data.get("key").is_some());
        assert!(event.data.get("modifiers").is_some());
        assert!(event.data.get("application").is_some());
    }
}