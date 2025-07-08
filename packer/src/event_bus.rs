// Event-driven architecture for loose coupling
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use std::collections::HashMap;
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronicleEvent {
    pub id: String,
    pub timestamp: i64,
    pub event_type: String,
    pub source: String,
    pub data: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle_event(&self, event: &ChronicleEvent) -> Result<(), Box<dyn std::error::Error>>;
    fn event_types(&self) -> Vec<String>;
}

pub struct EventBus {
    handlers: HashMap<String, Vec<Box<dyn EventHandler>>>,
    broadcast_tx: broadcast::Sender<ChronicleEvent>,
    command_tx: mpsc::Sender<BusCommand>,
}

#[derive(Debug)]
enum BusCommand {
    RegisterHandler { event_type: String, handler: Box<dyn EventHandler> },
    UnregisterHandler { event_type: String, handler_id: String },
    PublishEvent(ChronicleEvent),
    GetMetrics,
}

impl EventBus {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(10000);
        let (command_tx, mut command_rx) = mpsc::channel(1000);
        
        let mut bus = Self {
            handlers: HashMap::new(),
            broadcast_tx,
            command_tx,
        };
        
        // Spawn event processing loop
        let handlers = bus.handlers.clone();
        let broadcast_rx = bus.broadcast_tx.subscribe();
        
        tokio::spawn(async move {
            while let Some(command) = command_rx.recv().await {
                match command {
                    BusCommand::RegisterHandler { event_type, handler } => {
                        // Register handler logic
                    }
                    BusCommand::PublishEvent(event) => {
                        // Publish event to handlers
                        if let Some(event_handlers) = handlers.get(&event.event_type) {
                            for handler in event_handlers {
                                if let Err(e) = handler.handle_event(&event).await {
                                    tracing::error!("Handler error: {}", e);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
        
        bus
    }
    
    pub async fn register_handler<H: EventHandler + 'static>(&self, handler: H) {
        for event_type in handler.event_types() {
            let _ = self.command_tx.send(BusCommand::RegisterHandler {
                event_type,
                handler: Box::new(handler),
            }).await;
        }
    }
    
    pub async fn publish_event(&self, event: ChronicleEvent) {
        let _ = self.command_tx.send(BusCommand::PublishEvent(event)).await;
    }
}

// Example handlers for decoupling
pub struct StorageHandler;

#[async_trait]
impl EventHandler for StorageHandler {
    async fn handle_event(&self, event: &ChronicleEvent) -> Result<(), Box<dyn std::error::Error>> {
        // Store event to Parquet
        println!("Storing event: {}", event.id);
        Ok(())
    }
    
    fn event_types(&self) -> Vec<String> {
        vec!["keyboard".to_string(), "mouse".to_string(), "screen".to_string()]
    }
}

pub struct MetricsHandler;

#[async_trait]
impl EventHandler for MetricsHandler {
    async fn handle_event(&self, event: &ChronicleEvent) -> Result<(), Box<dyn std::error::Error>> {
        // Update metrics
        println!("Recording metrics for: {}", event.event_type);
        Ok(())
    }
    
    fn event_types(&self) -> Vec<String> {
        vec!["*".to_string()] // Handle all events
    }
}