#!/usr/bin/env node

const WebSocket = require('ws');

console.log('🔗 Testing direct WebSocket connection to dashboard...');

const ws = new WebSocket('ws://localhost:8080/ws');

let messageCount = 0;
let connected = false;

ws.on('open', () => {
    connected = true;
    console.log('✅ Connected to dashboard WebSocket');
    console.log('👂 Listening for messages...');
});

ws.on('message', (data) => {
    messageCount++;
    const message = data.toString();
    console.log(`📥 Message ${messageCount}:`, message);
    
    // Parse and analyze the message
    try {
        const parsed = JSON.parse(message);
        console.log(`   Type: ${parsed.msg_type || parsed.type || 'unknown'}`);
        if (parsed.msg_type === 'pool_sync' || parsed.msg_type === 'pool_swap') {
            console.log('🎉 POOL EVENT DETECTED!', JSON.stringify(parsed, null, 2));
        }
    } catch (e) {
        console.log('   Raw message (not JSON)');
    }
});

ws.on('error', (err) => {
    console.log('❌ WebSocket Error:', err.message);
});

ws.on('close', () => {
    console.log('🔌 WebSocket connection closed');
    if (connected) {
        console.log(`📊 Total messages received: ${messageCount}`);
    }
});

// Test for 10 seconds
setTimeout(() => {
    if (messageCount === 0) {
        console.log('⚠️  NO MESSAGES RECEIVED in 10 seconds');
        console.log('   This indicates polygon collector is not sending data');
    }
    ws.close();
}, 10000);

console.log('⏱️  Testing for 10 seconds...');