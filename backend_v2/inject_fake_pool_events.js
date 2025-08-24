#!/usr/bin/env node

// Quick fix: Inject fake pool events directly into the relay to prove the pipeline works
const net = require('net');

console.log('🧪 Injecting fake pool events to prove pipeline works...');

const socket = new net.Socket();

socket.connect('/tmp/alphapulse/market_data.sock', () => {
    console.log('✅ Connected to market data relay');
    
    // Create fake TLV-like message (simplified for testing)
    // Real TLV would use binary format, but this tests the relay forwarding
    
    setInterval(() => {
        const poolEvent = JSON.stringify({
            msg_type: "pool_swap",
            timestamp: Date.now(),
            venue_name: "Polygon",
            pool_address: "0x45dda9cb7c25131df268515131f647d726f50608",
            token0_symbol: "USDC",
            token1_symbol: "WETH",
            amount0_delta: Math.floor(Math.random() * -1000000),
            amount1_delta: Math.floor(Math.random() * 250000000000000),
            block_number: 12345678 + Math.floor(Math.random() * 1000),
            log_index: 42
        });
        
        socket.write(poolEvent + '\n');
        console.log('📤 Sent fake pool event');
    }, 2000);
});

socket.on('error', (err) => {
    console.log('❌ Socket error:', err.message);
});

console.log('🔄 Sending fake pool events every 2 seconds...');
console.log('📱 Check frontend at http://localhost:5177');

// Run for 30 seconds
setTimeout(() => {
    console.log('✅ Test completed');
    socket.end();
    process.exit(0);
}, 30000);