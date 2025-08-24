#!/usr/bin/env rust-script
//! Debug what subscription was actually sent
use std::str;

fn main() {
    // Hex payload from the trace log
    let hex_payload = "7b226964223a312c226a736f6e727063223a22322e30222c226d6574686f64223a226574685f737562736372696265222c22706172616d73223a5b226c6f6773222c7b22746f70696373223a5b5b22307863343230373966393461363335306437653632333566323931373439323466393238636332616338313865623634666564383030346531313566626363613637222c22307837613533303830626134313431353862653765633639623938376235666237643037646565313031666538353438386630383533616531363233396430626465222c22307830633339366364393839613339663434353962356661316165643661396138646364626334353930386163666436376530323863643536386461393839383263222c22307833303637303438626565653331623235623266313638316638386461633833386338626261333661663235626662326237636637343733613538343765333566222c22307831633431316539613936653037313234316332663231663737323662313761653839653363616234633738626535306530363262303361396666666262616431222c22307864646632353261643162653263383962363963326230363866633337386461613935326261376631363363346131313632386635356134646635323362336566222c22307838633562653165356562656337643562643134663731343237643165383466336464303331346330663762323239316535623230306163386337633362393235222c22307837383363636131633034313264643064363935653738343536386339366461326539633232666639383933353761326538623164396232623465366237313138222c22307830643336343862643066366261383031333461333362613932373561633538356439643331356630616438333535636464656664653331616661323864306539225d5d7d5d7d";

    // Decode hex to bytes
    let bytes = hex::decode(hex_payload).unwrap();
    let json_str = str::from_utf8(&bytes).unwrap();

    println!("Subscription sent by collector:");
    println!("{}", json_str);

    // Parse and pretty print
    let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
    println!("\nFormatted subscription:");
    println!("{}", serde_json::to_string_pretty(&parsed).unwrap());

    // Extract and decode the event signatures
    if let Some(params) = parsed.get("params") {
        if let Some(params_array) = params.as_array() {
            if params_array.len() > 1 {
                if let Some(filter) = params_array[1].as_object() {
                    if let Some(topics) = filter.get("topics") {
                        if let Some(topics_array) = topics.as_array() {
                            if !topics_array.is_empty() {
                                if let Some(signatures) = topics_array[0].as_array() {
                                    println!("\nEvent signatures in subscription:");
                                    for (i, sig) in signatures.iter().enumerate() {
                                        if let Some(sig_str) = sig.as_str() {
                                            println!("{}. {}", i + 1, sig_str);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
