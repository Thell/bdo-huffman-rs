#![allow(unused)]

use std::hint::black_box;

use common;

use baseline;

use nested_box;
use nested_unsafe_box;

use flat_index;
use flat_ptr;
use flat_unsafe_ptr;

use table_index;
use table_ptr;
use table_unsafe_ptr;

use table_single_index;
use table_single_unsafe_ptr;

use fsm;
use fsm_2channel;
use fsm_unsafe;
use fsm_unsafe_2channel;

fn main() {
    divan::main();
}

#[divan::bench(sample_count = 10_000)]
fn all_samples_mtable(bencher: divan::Bencher) {
    let mut samples = Vec::new();
    for sample in common::test_cases::SAMPLE_CASES {
        samples.push(sample.request());
    }
    let mut encoded_len = 0;
    let mut decoded_len = 0;
    for content in samples.iter() {
        let packet = common::packet::Packet::new(&content);
        encoded_len += packet.encoded_bytes_len;
        decoded_len += packet.decoded_bytes_len;
    }
    println!("\ntotal encoded_len: {}", encoded_len);
    println!("total decoded_len: {}", decoded_len);
    bencher.bench_local(move || {
        for content in samples.iter() {
            let packet = common::packet::Packet::new(&content);
            if packet.encoded_bytes_len <= 128 {
                black_box(flat_unsafe_ptr::decode_packet(&content));
            } else {
                black_box(table_unsafe_ptr::decode_packet(&content));
            }
        }
    });
}

#[divan::bench(sample_count = 10_000)]
fn all_samples_fsm(bencher: divan::Bencher) {
    let mut samples = Vec::new();
    for sample in common::test_cases::SAMPLE_CASES {
        samples.push(sample.request());
    }
    let mut encoded_len = 0;
    let mut decoded_len = 0;
    for content in samples.iter() {
        let packet = common::packet::Packet::new(&content);
        encoded_len += packet.encoded_bytes_len;
        decoded_len += packet.decoded_bytes_len;
    }
    println!("\ntotal encoded_len: {}", encoded_len);
    println!("total decoded_len: {}", decoded_len);
    bencher.bench_local(move || {
        for content in samples.iter() {
            let packet = common::packet::Packet::new(&content);
            if packet.encoded_bytes_len <= 128 {
                black_box(flat_unsafe_ptr::decode_packet(&content));
            } else if packet.encoded_bytes_len >= 40_000 {
                black_box(fsm_unsafe_2channel::decode_packet(&content));
            } else {
                black_box(table_unsafe_ptr::decode_packet(&content));
            }
        }
    });
}
