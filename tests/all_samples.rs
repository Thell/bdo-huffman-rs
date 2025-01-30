use common::packet::Packet;
use common::test_cases::*;

macro_rules! generate_test_cases {
    ($crate_name:ident) => {
        paste::paste! {
            #[test]
            fn [<all_samples_baseline_vs_ $crate_name>]() {
                for (i, case) in SAMPLE_CASES.iter().enumerate() {
                    let content = &case.request();
                    let expected_result = baseline::decode_packet(content);
                    let result = $crate_name::decode_packet(&content);
                    if expected_result != result {
                        println!(" failed SAMPLES_CASES[{}]: {:?}", i, case.name);
                        let packet = &Packet::new(content);
                        println!(" input bytes len: {}", packet.encoded_bytes_len);
                        println!("output bytes len: {}", packet.decoded_bytes_len);
                    }
                    assert_eq!(result, expected_result);
                }
            }
        }
    };
}

generate_test_cases!(flat_index);
generate_test_cases!(flat_ptr);
generate_test_cases!(flat_unsafe_ptr);
generate_test_cases!(nested_box);
generate_test_cases!(nested_unsafe_box);
generate_test_cases!(table_index);
generate_test_cases!(table_ptr);
generate_test_cases!(table_unsafe_ptr);
generate_test_cases!(fsm);
generate_test_cases!(fsm_2channel);
generate_test_cases!(fsm_3channel);
generate_test_cases!(fsm_4channel);
generate_test_cases!(fsm_unsafe);
generate_test_cases!(fsm_unsafe_2channel);
generate_test_cases!(fsm_unsafe_3channel);
generate_test_cases!(fsm_unsafe_4channel);
generate_test_cases!(fsm_unsafe_5channel);
