use common::test_cases::*;

macro_rules! generate_test_cases {
    ($crate_name:ident) => {
        paste::paste! {
            #[test]
            fn [<all_samples_baseline_vs_ $crate_name>]() {
                for case in SAMPLE_CASES {
                    let content = &case.request();
                    let expected_result = baseline::decode_packet(content);
                    let result = $crate_name::decode_packet(&content);
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
generate_test_cases!(state_table_index);
generate_test_cases!(state_table_unsafe);
