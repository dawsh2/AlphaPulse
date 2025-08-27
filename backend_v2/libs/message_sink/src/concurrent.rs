//! Concurrent Batch Processing for High-Throughput MessageSink
//!
//! Enables parallel message processing while maintaining ordering guarantees
//! where needed, optimizing for AlphaPulse's >1M msg/s throughput requirements.

use crate::{BatchResult, Message, MessageSink, SinkError, MessagePriority};
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::task::JoinHandle;

/// Configuration for concurrent batch processing
#[derive(Debug, Clone)]
pub struct ConcurrentConfig {
    /// Maximum number of concurrent send operations
    pub max_concurrency: usize,
    /// Size of batches to process concurrently
    pub batch_size: usize,
    /// Whether to preserve message ordering
    pub preserve_ordering: bool,
    /// Maximum messages to buffer before applying backpressure
    pub buffer_size: usize,
    /// Timeout for individual send operations (milliseconds)
    pub send_timeout_ms: u64,
}

impl Default for ConcurrentConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 10,
            batch_size: 100,
            preserve_ordering: false,
            buffer_size: 10_000,
            send_timeout_ms: 5_000,
        }
    }
}

impl ConcurrentConfig {
    /// High-throughput configuration for maximum performance
    pub fn high_throughput() -> Self {
        Self {
            max_concurrency: 50,
            batch_size: 1000,
            preserve_ordering: false,
            buffer_size: 100_000,
            send_timeout_ms: 1_000,
        }
    }

    /// Ordered configuration that preserves message sequence
    pub fn ordered() -> Self {
        Self {
            max_concurrency: 1,
            batch_size: 100,
            preserve_ordering: true,
            buffer_size: 10_000,
            send_timeout_ms: 5_000,
        }
    }

    /// Low-latency configuration for time-sensitive messages
    pub fn low_latency() -> Self {
        Self {
            max_concurrency: 20,
            batch_size: 10,
            preserve_ordering: false,
            buffer_size: 1_000,
            send_timeout_ms: 100,
        }
    }
}

/// Concurrent batch processor wrapper for MessageSink
#[derive(Debug)]
pub struct ConcurrentBatchSink<T: MessageSink> {
    inner: Arc<T>,
    config: ConcurrentConfig,
    semaphore: Arc<Semaphore>,
    sender: mpsc::Sender<BatchJob>,
    processor_handle: JoinHandle<()>,
}

/// Internal batch job representation
#[derive(Debug)]
struct BatchJob {
    messages: Vec<Message>,
    result_sender: tokio::sync::oneshot::Sender<Result<BatchResult, SinkError>>,
}

impl<T: MessageSink + 'static> ConcurrentBatchSink<T> {
    /// Create new concurrent batch processor
    pub fn new(inner: T, config: ConcurrentConfig) -> Self {
        let inner = Arc::new(inner);
        let semaphore = Arc::new(Semaphore::new(config.max_concurrency));
        let (sender, receiver) = mpsc::channel(config.buffer_size);
        
        let processor_handle = tokio::spawn(Self::batch_processor(
            Arc::clone(&inner),
            receiver,
            Arc::clone(&semaphore),
            config.clone(),
        ));

        Self {
            inner,
            config,
            semaphore,
            sender,
            processor_handle,
        }
    }

    /// Background processor that handles batches concurrently
    async fn batch_processor(
        sink: Arc<T>,
        mut receiver: mpsc::Receiver<BatchJob>,
        semaphore: Arc<Semaphore>,
        config: ConcurrentConfig,
    ) {
        let mut pending_jobs = Vec::new();
        // Limit pending jobs to prevent unbounded memory growth
        let max_pending = config.batch_size * 10; // Allow up to 10x batch size to accumulate
        
        while let Some(job) = receiver.recv().await {
            // Check if we're at capacity before adding
            if pending_jobs.len() >= max_pending {
                // Process immediately to prevent memory growth
                let jobs_to_process = pending_jobs.drain(..).collect::<Vec<_>>();
                Self::process_jobs(sink.clone(), jobs_to_process, semaphore.clone(), &config).await;
            }
            
            pending_jobs.push(job);
            
            // Process when we have enough jobs or channel is empty
            if pending_jobs.len() >= config.batch_size || receiver.is_empty() {
                let jobs_to_process = pending_jobs.drain(..).collect::<Vec<_>>();
                Self::process_jobs(sink.clone(), jobs_to_process, semaphore.clone(), &config).await;
            }
        }
    }
    
    /// Process a batch of jobs either sequentially or concurrently
    async fn process_jobs(
        sink: Arc<T>,
        jobs_to_process: Vec<BatchJob>,
        semaphore: Arc<Semaphore>,
        config: &ConcurrentConfig,
    ) {
        if config.preserve_ordering {
            // Process sequentially to maintain order
            for job in jobs_to_process {
                let result = Self::process_single_batch(
                    Arc::clone(&sink),
                    job.messages,
                    config.send_timeout_ms,
                ).await;
                let _ = job.result_sender.send(result);
            }
        } else {
            // Process concurrently for maximum throughput
            let mut handles = Vec::new();
            
            for job in jobs_to_process {
                let sink = Arc::clone(&sink);
                let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();
                let timeout_ms = config.send_timeout_ms;
                
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Hold permit for duration
                    let result = Self::process_single_batch(
                        sink,
                        job.messages,
                        timeout_ms,
                    ).await;
                    let _ = job.result_sender.send(result);
                });
                
                handles.push(handle);
            }
            
            // Wait for all concurrent operations
            for handle in handles {
                let _ = handle.await;
            }
        }
    }

    /// Process a single batch with timeout
    async fn process_single_batch(
        sink: Arc<T>,
        messages: Vec<Message>,
        timeout_ms: u64,
    ) -> Result<BatchResult, SinkError> {
        let timeout = tokio::time::Duration::from_millis(timeout_ms);
        
        match tokio::time::timeout(timeout, sink.send_batch(messages)).await {
            Ok(result) => result,
            Err(_) => Err(SinkError::timeout(timeout_ms / 1000)),
        }
    }

    /// Split messages by priority for concurrent processing
    pub async fn send_batch_by_priority(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        let mut priority_groups = std::collections::HashMap::new();
        
        // Group messages by priority
        for message in messages {
            priority_groups
                .entry(message.metadata.priority)
                .or_insert_with(Vec::new)
                .push(message);
        }
        
        // Process each priority level concurrently
        let mut handles = Vec::new();
        let mut total_result = BatchResult::new(0);
        
        for (priority, msgs) in priority_groups {
            let batch_size = msgs.len();
            total_result.total += batch_size;
            
            let (tx, rx) = tokio::sync::oneshot::channel();
            let job = BatchJob {
                messages: msgs,
                result_sender: tx,
            };
            
            // Higher priority gets processed first
            if priority == MessagePriority::Critical {
                // Process immediately without queuing
                let sink = Arc::clone(&self.inner);
                let timeout_ms = self.config.send_timeout_ms;
                
                let handle = tokio::spawn(async move {
                    Self::process_single_batch(sink, job.messages, timeout_ms).await
                });
                
                handles.push((rx, handle));
            } else {
                self.sender.send(job).await.map_err(|_| {
                    SinkError::send_failed("Concurrent processor channel closed")
                })?;
                handles.push((rx, tokio::spawn(async { Ok(BatchResult::new(0)) })));
            }
        }
        
        // Collect results
        for (rx, _) in handles {
            match rx.await {
                Ok(Ok(batch_result)) => {
                    total_result.succeeded += batch_result.succeeded;
                    total_result.failed.extend(batch_result.failed);
                }
                Ok(Err(e)) => {
                    return Err(e);
                }
                Err(_) => {
                    return Err(SinkError::send_failed("Result channel closed"));
                }
            }
        }
        
        Ok(total_result)
    }
}

#[async_trait]
impl<T: MessageSink + 'static> MessageSink for ConcurrentBatchSink<T> {
    async fn send(&self, message: Message) -> Result<(), SinkError> {
        // For single messages, bypass batching if low latency
        if self.config.batch_size == 1 {
            self.inner.send(message).await
        } else {
            // Queue for batch processing
            let (tx, rx) = tokio::sync::oneshot::channel();
            let job = BatchJob {
                messages: vec![message],
                result_sender: tx,
            };
            
            self.sender.send(job).await.map_err(|_| {
                SinkError::send_failed("Concurrent processor channel closed")
            })?;
            
            match rx.await {
                Ok(Ok(result)) if result.is_complete_success() => Ok(()),
                Ok(Ok(result)) => {
                    // Extract first error from batch result
                    if let Some((_, error)) = result.failed.into_iter().next() {
                        Err(error)
                    } else {
                        Ok(())
                    }
                }
                Ok(Err(e)) => Err(e),
                Err(_) => Err(SinkError::send_failed("Result channel closed")),
            }
        }
    }

    async fn send_batch(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        if messages.is_empty() {
            return Ok(BatchResult::new(0));
        }

        let (tx, rx) = tokio::sync::oneshot::channel();
        let job = BatchJob {
            messages,
            result_sender: tx,
        };
        
        self.sender.send(job).await.map_err(|_| {
            SinkError::send_failed("Concurrent processor channel closed")
        })?;
        
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(SinkError::send_failed("Result channel closed")),
        }
    }

    async fn send_batch_prioritized(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        self.send_batch_by_priority(messages).await
    }

    fn is_connected(&self) -> bool {
        self.inner.is_connected()
    }

    async fn connect(&self) -> Result<(), SinkError> {
        self.inner.connect().await
    }

    async fn disconnect(&self) -> Result<(), SinkError> {
        self.inner.disconnect().await
    }

    fn metadata(&self) -> crate::SinkMetadata {
        let mut metadata = self.inner.metadata();
        metadata.name = format!("{} (Concurrent x{})", metadata.name, self.config.max_concurrency);
        metadata
    }
}

impl<T: MessageSink> Drop for ConcurrentBatchSink<T> {
    fn drop(&mut self) {
        self.processor_handle.abort();
    }
}

/// Pipeline stages for multi-stage concurrent processing
#[derive(Debug)]
pub struct PipelinedSink<T: MessageSink> {
    stages: Vec<Arc<T>>,
    config: ConcurrentConfig,
}

impl<T: MessageSink + 'static> PipelinedSink<T> {
    /// Create a pipelined sink with multiple stages
    pub fn new(sinks: Vec<T>, config: ConcurrentConfig) -> Self {
        Self {
            stages: sinks.into_iter().map(Arc::new).collect(),
            config,
        }
    }

    /// Process messages through pipeline stages
    pub async fn process_pipeline(&self, messages: Vec<Message>) -> Result<BatchResult, SinkError> {
        let current_messages = messages;
        let mut total_result = BatchResult::new(current_messages.len());
        
        for (stage_idx, stage) in self.stages.iter().enumerate() {
            let stage_result = stage.send_batch(current_messages.clone()).await?;
            
            if stage_idx == self.stages.len() - 1 {
                // Final stage result is the overall result
                total_result = stage_result;
            } else if !stage_result.is_complete_success() {
                // Stop pipeline if any stage fails
                return Ok(stage_result);
            }
            
            // Could transform messages between stages here if needed
        }
        
        Ok(total_result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::CollectorSink;
    use crate::MessageMetadata;
    
    #[tokio::test]
    async fn test_concurrent_batch_basic() {
        let sink = CollectorSink::new();
        sink.connect().await.unwrap();
        
        let config = ConcurrentConfig {
            max_concurrency: 2,
            batch_size: 2,
            ..Default::default()
        };
        
        let concurrent_sink = ConcurrentBatchSink::new(sink, config);
        
        let msg = Message::new_unchecked(b"test".to_vec());
        concurrent_sink.send(msg).await.unwrap();
        
        // Give processor time to handle
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        assert_eq!(concurrent_sink.inner.message_count(), 1);
    }
    
    #[tokio::test]
    async fn test_concurrent_batch_multiple() {
        let sink = CollectorSink::new();
        sink.connect().await.unwrap();
        
        let config = ConcurrentConfig::high_throughput();
        let concurrent_sink = ConcurrentBatchSink::new(sink, config);
        
        let messages: Vec<_> = (0..100)
            .map(|i| Message::new_unchecked(format!("msg{}", i).into_bytes()))
            .collect();
        
        let result = concurrent_sink.send_batch(messages).await.unwrap();
        assert!(result.is_complete_success());
        assert_eq!(result.succeeded, 100);
        
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        assert_eq!(concurrent_sink.inner.message_count(), 100);
    }
    
    #[tokio::test]
    async fn test_priority_processing() {
        let sink = CollectorSink::new();
        sink.connect().await.unwrap();
        
        let concurrent_sink = ConcurrentBatchSink::new(sink, ConcurrentConfig::default());
        
        let messages = vec![
            Message::with_metadata(
                b"low".to_vec(),
                MessageMetadata::new().with_priority(MessagePriority::Low)
            ).unwrap(),
            Message::with_metadata(
                b"critical".to_vec(),
                MessageMetadata::new().with_priority(MessagePriority::Critical)
            ).unwrap(),
            Message::with_metadata(
                b"normal".to_vec(),
                MessageMetadata::new().with_priority(MessagePriority::Normal)
            ).unwrap(),
        ];
        
        let result = concurrent_sink.send_batch_by_priority(messages).await.unwrap();
        assert_eq!(result.total, 3);
    }
    
    #[tokio::test]
    async fn test_pipeline_processing() {
        let sink1 = CollectorSink::new();
        sink1.connect().await.unwrap();
        let sink2 = CollectorSink::new();
        sink2.connect().await.unwrap();
        
        let pipeline = PipelinedSink::new(vec![sink1, sink2], ConcurrentConfig::default());
        
        let messages = vec![
            Message::new_unchecked(b"msg1".to_vec()),
            Message::new_unchecked(b"msg2".to_vec()),
        ];
        
        let result = pipeline.process_pipeline(messages).await.unwrap();
        assert!(result.is_complete_success());
    }
}