#!/usr/bin/env node

const WebSocket = require('ws');

console.log('🔗 Testing direct connection to Polygon WebSocket...');

const ws = new WebSocket('wss://polygon-mainnet.public.blastapi.io');

ws.on('open', () => {
    console.log('✅ Connected to BlastAPI Polygon WebSocket');
    
    // Send the same subscription that our Rust code sends
    const subscription = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": [
            "logs",
            {
                "topics": [[
                    "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822", // swap
                    "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1"  // sync
                ]]
            }
        ]
    };
    
    console.log('📤 Sending subscription:', JSON.stringify(subscription, null, 2));
    ws.send(JSON.stringify(subscription));
});

ws.on('message', (data) => {
    const message = data.toString();
    console.log('📥 Received:', message);
    
    try {
        const parsed = JSON.parse(message);
        if (parsed.method === 'eth_subscription') {
            console.log('🎉 DEX EVENT RECEIVED!');
            console.log('   Event data:', JSON.stringify(parsed.params, null, 2));
        }
    } catch (e) {
        console.log('   (Not JSON)');
    }
});

ws.on('error', (err) => {
    console.log('❌ WebSocket Error:', err.message);
});

ws.on('close', () => {
    console.log('🔌 WebSocket connection closed');
});

// Test for 30 seconds to see if we get any DEX events
setTimeout(() => {
    console.log('⏱️  Test completed - closing connection');
    ws.close();
}, 30000);

console.log('⏱️  Testing for 30 seconds...');