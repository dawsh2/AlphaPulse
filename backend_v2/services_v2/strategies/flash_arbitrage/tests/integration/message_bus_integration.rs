//! Integration tests for message bus communication

use alphapulse_flash_arbitrage::pool_state::PoolStateManager;
use alphapulse_types::{
    RelayDomain, SourceType, TLVType,
};
use alphapulse_codec::TLVMessageBuilder;
// TODO: InMemoryMessageBus needs to be imported from the correct location
use std::sync::Arc;

#[tokio::test]
async fn test_message_bus_subscription() {
    let bus = Arc::new(InMemoryMessageBus::new());

    // Create receiver for market data
    let mut receiver = bus.create_receiver(RelayDomain::MarketData).await;

    // Send a message
    let mut builder = TLVMessageBuilder::new(SourceType::BinanceCollector, 12345);
    builder.add_tlv(TLVType::Trade, &[0u8; 48]).unwrap();
    let message = builder.build().unwrap();

    // Get sender for market data domain
    let sender = bus.get_or_create_channel(RelayDomain::MarketData).await;
    sender.send(message.clone()).unwrap();

    // Should receive the message
    if let Some(received) = receiver.recv().await {
        assert_eq!(received, message);
    } else {
        panic!("Did not receive message");
    }
}

#[tokio::test]
async fn test_multi_domain_routing() {
    let bus = Arc::new(InMemoryMessageBus::new());

    // Create receivers for different domains
    let mut market_receiver = bus.create_receiver(RelayDomain::MarketData).await;
    let mut signal_receiver = bus.create_receiver(RelayDomain::Signal).await;

    // Send market data message
    let mut market_builder = TLVMessageBuilder::new(SourceType::PolygonCollector, 100);
    market_builder.add_tlv(TLVType::Trade, &[1u8; 48]).unwrap();
    let market_msg = market_builder.build().unwrap();

    let market_sender = bus.get_or_create_channel(RelayDomain::MarketData).await;
    market_sender.send(market_msg.clone()).unwrap();

    // Send signal message
    let mut signal_builder = TLVMessageBuilder::new(SourceType::FlashArbitrageStrategy, 200);
    signal_builder
        .add_tlv(TLVType::ExecutionControl, &[2u8; 32])
        .unwrap();
    let signal_msg = signal_builder.build().unwrap();

    let signal_sender = bus.get_or_create_channel(RelayDomain::Signal).await;
    signal_sender.send(signal_msg.clone()).unwrap();

    // Each receiver should only get its domain's messages
    tokio::select! {
        Some(msg) = market_receiver.recv() => {
            assert_eq!(msg, market_msg);
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
            panic!("Market receiver timeout");
        }
    }

    tokio::select! {
        Some(msg) = signal_receiver.recv() => {
            assert_eq!(msg, signal_msg);
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
            panic!("Signal receiver timeout");
        }
    }
}

#[tokio::test]
async fn test_high_throughput_messaging() {
    let bus = Arc::new(InMemoryMessageBus::new());
    let mut receiver = bus.create_receiver(RelayDomain::MarketData).await;
    let sender = bus.get_or_create_channel(RelayDomain::MarketData).await;

    // Send many messages rapidly
    let num_messages = 1000;
    for i in 0..num_messages {
        let mut builder = TLVMessageBuilder::new(SourceType::BinanceCollector, i);
        builder.add_tlv(TLVType::Trade, &[i as u8; 48]).unwrap();
        let message = builder.build().unwrap();
        sender.send(message).unwrap();
    }

    // Should receive all messages
    let mut received_count = 0;
    let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(1));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Some(_msg) = receiver.recv() => {
                received_count += 1;
                if received_count == num_messages {
                    break;
                }
            }
            _ = &mut timeout => {
                break;
            }
        }
    }

    assert_eq!(received_count, num_messages);
}
