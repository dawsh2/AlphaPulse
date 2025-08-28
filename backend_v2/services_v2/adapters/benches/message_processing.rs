//! Performance benchmarks for message processing

use torq_codec::{InstrumentId, TLVHeader, TLVMessage, TLVType, VenueId};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde_json::json;

/// Benchmark JSON parsing performance
fn bench_json_parsing(c: &mut Criterion) {
    let json_trade = json!({
        "e": "trade",
        "E": 1700000000000u64,
        "s": "BTCUSDT",
        "t": 12345,
        "p": "45123.50",
        "q": "0.12345678",
        "T": 1700000000000u64,
        "m": false
    })
    .to_string();

    c.bench_function("json_parse_trade", |b| {
        b.iter(|| {
            let value: serde_json::Value = serde_json::from_str(&json_trade).unwrap();
            black_box(value);
        });
    });
}

/// Benchmark TLV serialization
fn bench_tlv_serialization(c: &mut Criterion) {
    let trade = TradeTLV {
        venue: VenueId::Binance,
        instrument_id: InstrumentId::from(0x4254435553445400u64),
        price: 45_123_50000000,
        volume: 12345678,
        side: 0,
        timestamp_ns: 1_700_000_000_000_000_000,
    };

    c.bench_function("tlv_serialize_trade", |b| {
        b.iter(|| {
            let tlv = trade.to_tlv_message();
            black_box(tlv);
        });
    });
}

/// Benchmark TLV deserialization
fn bench_tlv_deserialization(c: &mut Criterion) {
    let trade = TradeTLV {
        venue: VenueId::Binance,
        instrument_id: InstrumentId::from(0x4254435553445400u64),
        price: 45_123_50000000,
        volume: 12345678,
        side: 0,
        timestamp_ns: 1_700_000_000_000_000_000,
    };

    let tlv = trade.to_tlv_message();

    c.bench_function("tlv_deserialize_trade", |b| {
        b.iter(|| {
            let recovered = TradeTLV::from_tlv_message(&tlv).unwrap();
            black_box(recovered);
        });
    });
}

/// Benchmark complete roundtrip
fn bench_complete_roundtrip(c: &mut Criterion) {
    let trade = TradeTLV {
        venue: VenueId::Binance,
        instrument_id: InstrumentId::from(0x4254435553445400u64),
        price: 45_123_50000000,
        volume: 12345678,
        side: 0,
        timestamp_ns: 1_700_000_000_000_000_000,
    };

    c.bench_function("tlv_complete_roundtrip", |b| {
        b.iter(|| {
            let tlv = trade.to_tlv_message();
            let recovered = TradeTLV::from_tlv_message(&tlv).unwrap();
            black_box(recovered);
        });
    });
}

/// Benchmark throughput with different message sizes
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    for size in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let trades: Vec<TradeTLV> = (0..size)
                .map(|i| TradeTLV {
                    venue: VenueId::Binance,
                    instrument_id: InstrumentId::from(i as u64),
                    price: 45_000_00000000 + (i as i64 * 100000000),
                    volume: 1_00000000,
                    side: (i % 2) as u8,
                    timestamp_ns: 1_700_000_000_000_000_000 + i as u64,
                })
                .collect();

            b.iter(|| {
                for trade in &trades {
                    let tlv = trade.to_tlv_message();
                    let recovered = TradeTLV::from_tlv_message(&tlv).unwrap();
                    black_box(recovered);
                }
            });
        });
    }

    group.finish();
}

/// Benchmark decimal conversion
fn bench_decimal_conversion(c: &mut Criterion) {
    let price_strings = vec!["45123.50", "0.00000001", "999999.99999999", "1.23456789"];

    c.bench_function("decimal_string_to_i64", |b| {
        b.iter(|| {
            for price_str in &price_strings {
                let price: f64 = price_str.parse().unwrap();
                let fixed_point = (price * 1e8) as i64;
                black_box(fixed_point);
            }
        });
    });
}

// Test structures
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(zerocopy::AsBytes, zerocopy::FromBytes, zerocopy::FromZeroes)]
#[repr(C)]
struct TradeTLV {
    venue: VenueId,
    instrument_id: InstrumentId,
    price: i64,
    volume: i64,
    side: u8,
    timestamp_ns: u64,
}

impl TradeTLV {
    fn to_tlv_message(&self) -> TLVMessage {
        let mut payload = Vec::with_capacity(42);

        payload.push(self.venue as u8);
        payload.extend_from_slice(&self.instrument_id.to_le_bytes());
        payload.extend_from_slice(&self.price.to_le_bytes());
        payload.extend_from_slice(&self.volume.to_le_bytes());
        payload.push(self.side);
        payload.extend_from_slice(&self.timestamp_ns.to_le_bytes());

        let checksum = calculate_checksum(&payload);

        TLVMessage {
            header: TLVHeader {
                magic: 0xDEADBEEF, // Protocol V2 standard magic number
                tlv_type: TLVType::Trade,
                payload_len: payload.len() as u8,
                checksum,
            },
            payload,
        }
    }

    fn from_tlv_message(msg: &TLVMessage) -> Result<Self, String> {
        if msg.payload.len() != 42 {
            return Err("Invalid payload size".to_string());
        }

        Ok(TradeTLV {
            venue: VenueId::try_from(msg.payload[0]).map_err(|_| "Invalid venue")?,
            instrument_id: InstrumentId::from(u64::from_le_bytes(
                msg.payload[1..9].try_into().unwrap(),
            )),
            price: i64::from_le_bytes(msg.payload[9..17].try_into().unwrap()),
            volume: i64::from_le_bytes(msg.payload[17..25].try_into().unwrap()),
            side: msg.payload[25],
            timestamp_ns: u64::from_le_bytes(msg.payload[26..34].try_into().unwrap()),
        })
    }
}

fn calculate_checksum(payload: &[u8]) -> u8 {
    payload.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

criterion_group!(
    benches,
    bench_json_parsing,
    bench_tlv_serialization,
    bench_tlv_deserialization,
    bench_complete_roundtrip,
    bench_throughput,
    bench_decimal_conversion
);

criterion_main!(benches);
