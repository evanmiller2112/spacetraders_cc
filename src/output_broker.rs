use std::collections::VecDeque;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};

/// Central output broker that manages ALL print statements
/// Provides filtering, batching, and verbosity control
#[derive(Clone)]
pub struct OutputBroker {
    sender: mpsc::UnboundedSender<OutputRequest>,
}

/// Output request that gets queued through the broker
pub struct OutputRequest {
    pub level: OutputLevel,
    pub message: String,
    pub timestamp: Instant,
    pub response_sender: Option<oneshot::Sender<()>>,
}

/// Output levels for filtering
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum OutputLevel {
    Error,      // Always shown - critical errors
    Summary,    // Level 0+ - contract/ship status summaries  
    Info,       // Level 1+ - basic operational info
    Debug,      // Level 2+ - detailed debug info
    Trace,      // Level 2+ - full trace information
}

/// Internal broker state
struct BrokerState {
    verbosity_level: u8,
    pending_summaries: VecDeque<String>,
    last_summary_flush: Instant,
    summary_interval: Duration,
}

impl OutputBroker {
    /// Create a new output broker and start the background processing loop
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        // Spawn the broker worker task
        tokio::spawn(Self::broker_worker(receiver));
        
        Self { sender }
    }
    
    /// Submit an output request through the broker
    pub async fn output(&self, level: OutputLevel, message: String) {
        let request = OutputRequest {
            level,
            message,
            timestamp: Instant::now(),
            response_sender: None,
        };
        
        // Send to broker queue - ignore if broker is down
        let _ = self.sender.send(request);
    }
    
    /// Submit an output request and wait for it to be processed
    pub async fn output_and_wait(&self, level: OutputLevel, message: String) {
        let (response_sender, response_receiver) = oneshot::channel();
        
        let request = OutputRequest {
            level,
            message,
            timestamp: Instant::now(),
            response_sender: Some(response_sender),
        };
        
        // Send to broker queue
        if self.sender.send(request).is_ok() {
            // Wait for processing to complete
            let _ = response_receiver.await;
        }
    }
    
    /// Flush any pending summary information immediately
    pub async fn flush_summaries(&self) {
        self.output(OutputLevel::Summary, "FLUSH_SUMMARIES".to_string()).await;
    }
    
    /// Update verbosity level
    pub async fn set_verbosity_level(&self, level: u8) {
        self.output(OutputLevel::Debug, format!("SET_VERBOSITY_{}", level)).await;
    }
    
    /// Background worker that processes all output requests with filtering
    async fn broker_worker(mut receiver: mpsc::UnboundedReceiver<OutputRequest>) {
        let mut state = BrokerState {
            verbosity_level: crate::verbosity::get_verbosity_level(),
            pending_summaries: VecDeque::new(),
            last_summary_flush: Instant::now(),
            summary_interval: Duration::from_secs(30), // Flush summaries every 30s
        };
        
        println!("📺 Output Broker started - centralizing ALL print statements");
        
        while let Some(request) = receiver.recv().await {
            Self::handle_output(&mut state, request).await;
            
            // Check if we should flush summaries periodically
            if state.last_summary_flush.elapsed() > state.summary_interval {
                Self::flush_pending_summaries(&mut state).await;
            }
        }
        
        // Flush any remaining summaries on shutdown
        Self::flush_pending_summaries(&mut state).await;
        println!("⚠️ Output Broker stopped");
    }
    
    /// Handle a single output request with filtering and batching
    async fn handle_output(state: &mut BrokerState, request: OutputRequest) {
        // Handle special control messages
        if request.message == "FLUSH_SUMMARIES" {
            Self::flush_pending_summaries(state).await;
            if let Some(sender) = request.response_sender {
                let _ = sender.send(());
            }
            return;
        }
        
        if request.message.starts_with("SET_VERBOSITY_") {
            if let Ok(level) = request.message[14..].parse::<u8>() {
                state.verbosity_level = level;
                println!("📢 Output verbosity level: {} (0=summary only, 1=basic, 2=full)", level);
            }
            if let Some(sender) = request.response_sender {
                let _ = sender.send(());
            }
            return;
        }
        
        // Apply verbosity filtering
        let should_show = match request.level {
            OutputLevel::Error => true,  // Always show errors
            OutputLevel::Summary => true, // Always show summaries at level 0+
            OutputLevel::Info => state.verbosity_level >= 1,
            OutputLevel::Debug => state.verbosity_level >= 2,
            OutputLevel::Trace => state.verbosity_level >= 2,
        };
        
        if !should_show {
            if let Some(sender) = request.response_sender {
                let _ = sender.send(());
            }
            return;
        }
        
        // Handle summary batching vs immediate output
        match request.level {
            OutputLevel::Summary => {
                // Batch summaries for periodic output
                state.pending_summaries.push_back(request.message);
                
                // Limit summary buffer size
                if state.pending_summaries.len() > 100 {
                    state.pending_summaries.pop_front();
                }
            }
            _ => {
                // Output immediately for non-summary messages
                println!("{}", request.message);
            }
        }
        
        // Signal completion
        if let Some(sender) = request.response_sender {
            let _ = sender.send(());
        }
    }
    
    /// Flush all pending summary messages
    async fn flush_pending_summaries(state: &mut BrokerState) {
        if !state.pending_summaries.is_empty() {
            println!("\n🎖️ === CYCLE SUMMARY ===");
            
            while let Some(summary) = state.pending_summaries.pop_front() {
                println!("{}", summary);
            }
            
            println!("🎖️ === END SUMMARY ===\n");
            state.last_summary_flush = Instant::now();
        }
    }
}

/// Global output broker instance
static GLOBAL_BROKER: OnceLock<OutputBroker> = OnceLock::new();

/// Initialize the global output broker
pub fn init_output_broker() {
    GLOBAL_BROKER.get_or_init(|| OutputBroker::new());
}

/// Get the global output broker instance
pub fn get_output_broker() -> &'static OutputBroker {
    GLOBAL_BROKER.get_or_init(|| OutputBroker::new())
}

/// Global output macros that work anywhere
#[macro_export]
macro_rules! o_error {
    ($($arg:tt)*) => {{
        let broker = $crate::output_broker::get_output_broker();
        let message = format!($($arg)*);
        tokio::spawn(broker.output($crate::output_broker::OutputLevel::Error, message));
    }};
}

#[macro_export]
macro_rules! o_summary {
    ($($arg:tt)*) => {{
        let broker = $crate::output_broker::get_output_broker();
        let message = format!($($arg)*);
        tokio::spawn(broker.output($crate::output_broker::OutputLevel::Summary, message));
    }};
}

#[macro_export]
macro_rules! o_info {
    ($($arg:tt)*) => {{
        let broker = $crate::output_broker::get_output_broker();
        let message = format!($($arg)*);
        tokio::spawn(broker.output($crate::output_broker::OutputLevel::Info, message));
    }};
}

#[macro_export]
macro_rules! o_debug {
    ($($arg:tt)*) => {{
        let broker = $crate::output_broker::get_output_broker();
        let message = format!($($arg)*);
        tokio::spawn(broker.output($crate::output_broker::OutputLevel::Debug, message));
    }};
}

#[macro_export]
macro_rules! o_trace {
    ($($arg:tt)*) => {{
        let broker = $crate::output_broker::get_output_broker();
        let message = format!($($arg)*);
        tokio::spawn(broker.output($crate::output_broker::OutputLevel::Trace, message));
    }};
}