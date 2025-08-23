//! Topic-based pub-sub routing for coarse-grained filtering

use crate::{ConsumerId, RelayError, RelayResult, TopicConfig, TopicExtractionStrategy};
use dashmap::DashMap;
use protocol_v2::{MessageHeader, SourceType};
use std::collections::HashSet;
use tracing::{debug, info, warn};

/// Registry for topic-based message routing
pub struct TopicRegistry {
    /// Map of topics to subscriber lists
    topics: DashMap<String, HashSet<ConsumerId>>,
    /// Configuration for topic handling
    config: TopicConfig,
    /// Reverse mapping: consumer to topics
    consumer_topics: DashMap<ConsumerId, HashSet<String>>,
}

impl TopicRegistry {
    /// Create new topic registry
    pub fn new(config: &TopicConfig) -> RelayResult<Self> {
        let registry = Self {
            topics: DashMap::new(),
            config: config.clone(),
            consumer_topics: DashMap::new(),
        };

        // Initialize available topics
        for topic in &config.available {
            registry.topics.insert(topic.clone(), HashSet::new());
            info!("Initialized topic: {}", topic);
        }

        // Add default topic
        registry
            .topics
            .insert(config.default.clone(), HashSet::new());

        Ok(registry)
    }

    /// Subscribe a consumer to a topic
    pub fn subscribe(&self, consumer_id: ConsumerId, topic: &str) -> RelayResult<()> {
        // Check if topic exists or auto-discover is enabled
        if !self.topics.contains_key(topic) {
            if self.config.auto_discover {
                info!("Auto-discovering new topic: {}", topic);
                self.topics.insert(topic.to_string(), HashSet::new());
            } else {
                return Err(RelayError::TopicNotFound(topic.to_string()));
            }
        }

        // Add consumer to topic
        self.topics
            .entry(topic.to_string())
            .and_modify(|subscribers| {
                subscribers.insert(consumer_id.clone());
            });

        // Track consumer's topics
        self.consumer_topics
            .entry(consumer_id.clone())
            .and_modify(|topics| {
                topics.insert(topic.to_string());
            })
            .or_insert_with(|| {
                let mut topics = HashSet::new();
                topics.insert(topic.to_string());
                topics
            });

        debug!("Consumer {} subscribed to topic {}", consumer_id.0, topic);
        Ok(())
    }

    /// Unsubscribe a consumer from a topic
    pub fn unsubscribe(&self, consumer_id: &ConsumerId, topic: &str) -> RelayResult<()> {
        // Remove from topic subscribers
        if let Some(mut subscribers) = self.topics.get_mut(topic) {
            subscribers.remove(consumer_id);
            debug!(
                "Consumer {} unsubscribed from topic {}",
                consumer_id.0, topic
            );
        }

        // Remove from consumer's topic list
        if let Some(mut topics) = self.consumer_topics.get_mut(consumer_id) {
            topics.remove(topic);
        }

        Ok(())
    }

    /// Unsubscribe consumer from all topics
    pub fn unsubscribe_all(&self, consumer_id: &ConsumerId) -> RelayResult<()> {
        // Get all topics for this consumer
        if let Some(topics) = self.consumer_topics.remove(consumer_id) {
            // Remove from each topic
            for topic in topics.1 {
                if let Some(mut subscribers) = self.topics.get_mut(&topic) {
                    subscribers.remove(consumer_id);
                }
            }
            info!("Consumer {} unsubscribed from all topics", consumer_id.0);
        }

        Ok(())
    }

    /// Extract topic from message based on strategy
    pub fn extract_topic(
        &self,
        header: &MessageHeader,
        _tlv_payload: Option<&[u8]>,
        strategy: &TopicExtractionStrategy,
    ) -> RelayResult<String> {
        let topic = match strategy {
            TopicExtractionStrategy::SourceType => {
                // Map source type to topic
                self.source_type_to_topic(header.source)?
            }
            TopicExtractionStrategy::InstrumentVenue => {
                // TODO: Extract venue from TLV payload containing instrument ID
                // For now, use default topic
                warn!("InstrumentVenue extraction requires TLV parsing - using default");
                self.config.default.clone()
            }
            TopicExtractionStrategy::CustomField(field_id) => {
                // Look for custom TLV field
                // TODO: Parse TLVs to find custom field
                warn!("Custom field extraction not yet implemented");
                self.config.default.clone()
            }
            TopicExtractionStrategy::Fixed(topic) => {
                // Always use fixed topic
                topic.clone()
            }
        };

        Ok(topic)
    }

    /// Map source type to topic name
    fn source_type_to_topic(&self, source_type: u8) -> RelayResult<String> {
        let topic = match source_type {
            1 => "market_data_binance",
            2 => "market_data_kraken",
            3 => "market_data_coinbase",
            4 => "market_data_polygon",
            20 => "arbitrage_signals",
            21 => "market_maker_signals",
            22 => "trend_signals",
            40 => "execution_orders",
            41 => "risk_updates",
            42 => "execution_fills",
            _ => {
                debug!("Unknown source type {}, using default topic", source_type);
                return Ok(self.config.default.clone());
            }
        };

        Ok(topic.to_string())
    }

    /// Extract venue from instrument ID to create topic
    fn extract_venue_topic(&self, instrument_id: u64) -> RelayResult<String> {
        // Instrument ID format: [exchange:8][base:8][quote:8][type:8][venue:16][reserved:16]
        let venue = ((instrument_id >> 16) & 0xFFFF) as u16;

        let topic = match venue {
            1 => "market_data_uniswap_v2",
            2 => "market_data_uniswap_v3",
            3 => "market_data_sushiswap",
            4 => "market_data_quickswap",
            _ => {
                debug!("Unknown venue {}, using default topic", venue);
                return Ok(self.config.default.clone());
            }
        };

        Ok(topic.to_string())
    }

    /// Get all subscribers for a topic
    pub fn get_subscribers(&self, topic: &str) -> Vec<ConsumerId> {
        self.topics
            .get(topic)
            .map(|subscribers| subscribers.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get subscriber count for a topic
    pub fn subscriber_count(&self, topic: &str) -> usize {
        self.topics
            .get(topic)
            .map(|subscribers| subscribers.len())
            .unwrap_or(0)
    }

    /// List all available topics
    pub fn list_topics(&self) -> Vec<String> {
        self.topics
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Get topics for a consumer
    pub fn get_consumer_topics(&self, consumer_id: &ConsumerId) -> Vec<String> {
        self.consumer_topics
            .get(consumer_id)
            .map(|topics| topics.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get statistics about topic registry
    pub fn stats(&self) -> TopicStats {
        TopicStats {
            total_topics: self.topics.len(),
            total_consumers: self.consumer_topics.len(),
            total_subscriptions: self
                .consumer_topics
                .iter()
                .map(|entry| entry.value().len())
                .sum(),
        }
    }
}

/// Topic registry statistics
#[derive(Debug, Clone)]
pub struct TopicStats {
    pub total_topics: usize,
    pub total_consumers: usize,
    pub total_subscriptions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_subscription() {
        let config = TopicConfig {
            default: "default".to_string(),
            available: vec!["topic1".to_string(), "topic2".to_string()],
            auto_discover: true,
            extraction_strategy: TopicExtractionStrategy::SourceType,
        };

        let registry = TopicRegistry::new(&config).unwrap();
        let consumer = ConsumerId("test_consumer".to_string());

        // Subscribe to existing topic
        registry.subscribe(consumer.clone(), "topic1").unwrap();
        assert_eq!(registry.subscriber_count("topic1"), 1);

        // Subscribe to new topic (auto-discover)
        registry.subscribe(consumer.clone(), "topic3").unwrap();
        assert_eq!(registry.subscriber_count("topic3"), 1);

        // Check consumer's topics
        let topics = registry.get_consumer_topics(&consumer);
        assert_eq!(topics.len(), 2);

        // Unsubscribe from one topic
        registry.unsubscribe(&consumer, "topic1").unwrap();
        assert_eq!(registry.subscriber_count("topic1"), 0);

        // Unsubscribe from all
        registry.unsubscribe_all(&consumer).unwrap();
        assert_eq!(registry.subscriber_count("topic3"), 0);
    }

    #[test]
    fn test_topic_extraction() {
        let config = TopicConfig {
            default: "default".to_string(),
            available: vec![],
            auto_discover: false,
            extraction_strategy: TopicExtractionStrategy::SourceType,
        };

        let registry = TopicRegistry::new(&config).unwrap();

        let mut header = MessageHeader {
            magic: protocol_v2::MESSAGE_MAGIC,
            version: 1,
            message_type: 1,
            relay_domain: 1,
            source_type: 4, // Polygon collector
            sequence: 1,
            timestamp_ns: 0,
            instrument_id: 0,
            checksum: 0,
        };

        // Test source type extraction
        let topic = registry
            .extract_topic(&header, None, &TopicExtractionStrategy::SourceType)
            .unwrap();
        assert_eq!(topic, "market_data_polygon");

        // Test fixed topic
        let topic = registry
            .extract_topic(
                &header,
                None,
                &TopicExtractionStrategy::Fixed("fixed".to_string()),
            )
            .unwrap();
        assert_eq!(topic, "fixed");
    }
}
