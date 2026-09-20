use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use proptest::prelude::*;
use vidya_core::{
    attendance::{check_attendance_date, next_status, percent, AttendanceStatus},
    dates::{
        days_in_month, default_session_name, format_date_en, parse_date, parse_session_name, session_bounds,
    },
    error::{DomainError, ErrorKind},
    fees::{
        balance, fee_state, one_term_amount, receipt_number, session_due, term_fee, validate_payment,
        FeeInputs, FeePlan, FeeState, PayMode, PaymentCheck,
    },
    hlc::{Hlc, HlcClock},
    i18n::message,
    marks::{exam_result, grade_for, parse_mark, validate_grade_scale, GradeBand, MarkValue},
    money::{format_inr, parse_rupees, Rupees},
    roles::{Lang, Role},
    secrets::{format_session_token, format_temp_password},
    usernames::{fallback_username_base, unique_username, username_base_from_name},
    validation::{
        validate_cheque_reference, validate_class_name, validate_mobile, validate_new_password,
        validate_person_name, validate_reason, validate_receipt_prefix, validate_school_code, validate_udise,
        validate_upi_reference, validate_username,
    },
    words::{amount_in_words_en, receipt_words_en},
};

#[test]
fn domain_error_builder_and_messages() {
    let error = DomainError::validation("fees.error.over_balance")
        .field("amount")
        .param("balance", "₹9,100");
    assert_eq!(error.kind, ErrorKind::Validation);
    assert_eq!(error.field, Some("amount"));
    assert_eq!(error.to_string(), "fees.error.over_balance");
    assert_eq!(
        message(Lang::En, error.message_key, &error.params),
        "That is more than the balance of ₹9,100."
    );
}

#[test]
fn role_serialization_uses_wire_names() {
    assert_eq!(serde_json::to_string(&Role::Principal).unwrap(), "\"principal\"");
    assert_eq!(
        serde_json::to_string(&Role::Accountant).unwrap(),
        "\"accountant\""
    );
    assert_eq!(serde_json::to_string(&Role::Teacher).unwrap(), "\"teacher\"");
}

#[test]
fn money_format_vectors() {
    let vectors = [
        (0, "₹0"),
        (999, "₹999"),
        (1_000, "₹1,000"),
        (100_000, "₹1,00,000"),
        (12_345_678, "₹1,23,45,678"),
        (-500, "−₹500"),
    ];
    for (amount, expected) in vectors {
        assert_eq!(format_inr(Rupees(amount)), expected);
    }
}

#[test]
fn money_parse_vectors() {
    assert_eq!(parse_rupees("1,000"), Ok(Rupees(1_000)));
    assert_eq!(parse_rupees("₹ 500"), Ok(Rupees(500)));
    assert_eq!(parse_rupees(" 750 "), Ok(Rupees(750)));
    for invalid in ["", "-1", "2.50", "1000000001", "hello"] {
        assert_eq!(
            parse_rupees(invalid).unwrap_err().message_key,
            "money.error.invalid"
        );
    }
}

proptest! {
    #[test]
    fn money_round_trips(amount in 0_i64..=1_000_000_000) {
        let formatted = format_inr(Rupees(amount));
        prop_assert_eq!(parse_rupees(formatted.trim_start_matches('₹')), Ok(Rupees(amount)));
    }
}

#[test]
fn amount_in_words_vectors() {
    let vectors = [
        (0, "Zero"),
        (7, "Seven"),
        (15, "Fifteen"),
        (20, "Twenty"),
        (45, "Forty Five"),
        (100, "One Hundred"),
        (101, "One Hundred One"),
        (999, "Nine Hundred Ninety Nine"),
        (1_000, "One Thousand"),
        (1_005, "One Thousand Five"),
        (12_000, "Twelve Thousand"),
        (99_999, "Ninety Nine Thousand Nine Hundred Ninety Nine"),
        (100_000, "One Lakh"),
        (250_500, "Two Lakh Fifty Thousand Five Hundred"),
        (
            1_234_567,
            "Twelve Lakh Thirty Four Thousand Five Hundred Sixty Seven",
        ),
        (10_000_000, "One Crore"),
        (
            123_456_789,
            "Twelve Crore Thirty Four Lakh Fifty Six Thousand Seven Hundred Eighty Nine",
        ),
    ];
    for (amount, expected) in vectors {
        assert_eq!(amount_in_words_en(amount), expected);
    }
    assert_eq!(receipt_words_en(45), "Rupees Forty Five Only");
}

#[test]
fn date_and_session_vectors() {
    let leap_day = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
    assert_eq!(parse_date("2024-02-29"), Ok(leap_day));
    assert_eq!(
        format_date_en(NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()),
        "15 Sep 2026"
    );
    for invalid in ["2023-02-29", "15-09-2026", "2026-9-15", "2026/09/15"] {
        assert!(parse_date(invalid).is_err());
    }
    assert_eq!(days_in_month(2024, 2), 29);
    assert_eq!(days_in_month(2023, 2), 28);
    assert_eq!(days_in_month(2026, 13), 0);
    assert_eq!(
        default_session_name(NaiveDate::from_ymd_opt(2026, 3, 31).unwrap()),
        "2025-26"
    );
    assert_eq!(
        default_session_name(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()),
        "2026-27"
    );
    assert_eq!(parse_session_name("2026-27"), Ok((2026, 2027)));
    assert!(parse_session_name("2026-26").is_err());
    assert!(parse_session_name("26-27").is_err());
    assert!(parse_session_name("2026/27").is_err());
    assert_eq!(
        session_bounds("2026-27").unwrap(),
        (
            NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
            NaiveDate::from_ymd_opt(2027, 3, 31).unwrap(),
        )
    );
}

#[test]
fn person_name_validation_vectors() {
    for (input, expected) in [
        ("  Ravi   Kumar ", "Ravi Kumar"),
        ("सुनीता मिश्रा", "सुनीता मिश्रा"),
        ("Sierra D'Souza", "Sierra D'Souza"),
    ] {
        assert_eq!(validate_person_name(input, "name").unwrap(), expected);
    }
    for input in ["A", "Ravi_1", "--"] {
        assert!(validate_person_name(input, "name").is_err());
    }
}

#[test]
fn mobile_validation_vectors() {
    for valid in ["6123456789", "7987654321", "9999999999"] {
        assert!(validate_mobile(valid).is_ok());
    }
    for invalid in ["5123456789", "91234", "912345678a"] {
        assert!(validate_mobile(invalid).is_err());
    }
}

#[test]
fn udise_validation_vectors() {
    for valid in ["", "  ", "12345678901"] {
        assert!(validate_udise(valid).is_ok());
    }
    for invalid in ["1234567890", "123456789012", "1234567890a"] {
        assert!(validate_udise(invalid).is_err());
    }
}

#[test]
fn school_code_validation_vectors() {
    for valid in ["abc", "school1", "a12345678901"] {
        assert!(validate_school_code(valid).is_ok());
    }
    for invalid in ["ab", "School", "school-1"] {
        assert!(validate_school_code(invalid).is_err());
    }
}

#[test]
fn username_validation_vectors() {
    for valid in ["ab", "sunita", "teacher20"] {
        assert!(validate_username(valid).is_ok());
    }
    for invalid in ["a", "Sunita", "teacher_2"] {
        assert!(validate_username(invalid).is_err());
    }
}

#[test]
fn password_validation_vectors() {
    for valid in ["River!234", "safe pass 9", "मजबूतपासवर्ड9"] {
        assert!(validate_new_password(valid, "sunita", false).is_ok());
    }
    assert_eq!(
        validate_new_password("short", "sunita", false)
            .unwrap_err()
            .message_key,
        "password.too_short"
    );
    assert_eq!(
        validate_new_password("xxSUNITAxx", "sunita", false)
            .unwrap_err()
            .message_key,
        "password.contains_username"
    );
    assert_eq!(
        validate_new_password("River!234", "sunita", true)
            .unwrap_err()
            .message_key,
        "password.same_as_current"
    );
}

#[test]
fn receipt_prefix_validation_vectors() {
    for valid in ["PC", "T01", "AB12"] {
        assert!(validate_receipt_prefix(valid).is_ok());
    }
    for invalid in ["P", "ABCDE", "pc"] {
        assert!(validate_receipt_prefix(invalid).is_err());
    }
}

#[test]
fn class_name_validation_vectors() {
    for valid in ["V", "Class 10", "नर्सरी"] {
        assert!(validate_class_name(valid).is_ok());
    }
    for invalid in ["", "Class-X", "A very long class name"] {
        assert!(validate_class_name(invalid).is_err());
    }
}

#[test]
fn reason_validation_vectors() {
    for valid in ["Late fee", "  left school  ", "गलत रसीद"] {
        assert!(validate_reason(valid, 4).is_ok());
    }
    for invalid in ["", "abc", "   "] {
        assert!(validate_reason(invalid, 4).is_err());
    }
}

#[test]
fn upi_reference_validation_vectors() {
    for valid in ["000000000000", "123456789012", "999999999999"] {
        assert!(validate_upi_reference(valid).is_ok());
    }
    for invalid in ["123", "1234567890123", "12345678901a"] {
        assert!(validate_upi_reference(invalid).is_err());
    }
}

#[test]
fn cheque_reference_validation_vectors() {
    for valid in ["1234", " CHQ-9 ", "bank ref"] {
        assert!(validate_cheque_reference(valid).is_ok());
    }
    for invalid in ["", "123", "   "] {
        assert!(validate_cheque_reference(invalid).is_err());
    }
}

fn fee_inputs() -> FeeInputs {
    FeeInputs {
        plan: FeePlan {
            tuition: 2_400,
            exam: 300,
            other: 400,
        },
        terms: 3,
        transport_fee_per_term: 900,
        transport: true,
        rte: false,
        concession: 0,
    }
}

#[test]
fn fee_vectors() {
    let mut inputs = fee_inputs();
    assert_eq!(term_fee(&inputs.plan), 3_100);
    assert_eq!(one_term_amount(&inputs), 4_000);
    assert_eq!(session_due(&inputs), 12_000);
    inputs.concession = 1_000;
    assert_eq!(session_due(&inputs), 11_000);
    inputs.concession = 50_000;
    assert_eq!(session_due(&inputs), 0);
    inputs.rte = true;
    assert_eq!(session_due(&inputs), 0);
    inputs = fee_inputs();
    inputs.terms = 12;
    inputs.transport = false;
    assert_eq!(session_due(&inputs), 37_200);
    assert_eq!(fee_state(10_000, 0, true), FeeState::Rte);
    assert_eq!(fee_state(10_000, 10_000, false), FeeState::Paid);
    assert_eq!(fee_state(10_000, 500, false), FeeState::Part);
    assert_eq!(fee_state(10_000, 0, false), FeeState::Due);
    assert_eq!(receipt_number("PC", 1), "PC-0001");
    assert_eq!(receipt_number("PC", 10_000), "PC-10000");
}

fn payment(mode: PayMode, reference: &str) -> PaymentCheck<'_> {
    PaymentCheck {
        amount: 500,
        balance: 1_000,
        mode,
        reference,
        upi_reference_already_used: false,
        rte: false,
        student_active: true,
    }
}

#[test]
fn payment_validation_order_and_modes() {
    assert!(validate_payment(&payment(PayMode::Cash, "")).is_ok());
    assert!(validate_payment(&payment(PayMode::Upi, "123456789012")).is_ok());
    assert!(validate_payment(&payment(PayMode::Cheque, "CHQ1")).is_ok());
    let mut check = payment(PayMode::Cash, "");
    check.student_active = false;
    check.rte = true;
    check.amount = 0;
    assert_eq!(
        validate_payment(&check).unwrap_err().message_key,
        "fees.error.student_not_active"
    );
    check.student_active = true;
    assert_eq!(
        validate_payment(&check).unwrap_err().message_key,
        "fees.error.rte"
    );
    check.rte = false;
    assert_eq!(
        validate_payment(&check).unwrap_err().message_key,
        "fees.error.amount"
    );
    check.amount = 1;
    check.balance = 0;
    assert_eq!(
        validate_payment(&check).unwrap_err().message_key,
        "fees.error.no_balance"
    );
    check.balance = 0;
    check.amount = 2;
    assert_eq!(
        validate_payment(&check).unwrap_err().message_key,
        "fees.error.no_balance"
    );
    check.balance = 1;
    assert_eq!(
        validate_payment(&check).unwrap_err().message_key,
        "fees.error.over_balance"
    );
    let invalid_upi = payment(PayMode::Upi, "123");
    assert_eq!(
        validate_payment(&invalid_upi).unwrap_err().message_key,
        "fees.error.upi_reference"
    );
    let mut used_upi = payment(PayMode::Upi, "123456789012");
    used_upi.upi_reference_already_used = true;
    assert_eq!(
        validate_payment(&used_upi).unwrap_err().message_key,
        "fees.error.upi_used"
    );
    assert_eq!(
        validate_payment(&payment(PayMode::Cheque, "12"))
            .unwrap_err()
            .message_key,
        "fees.error.cheque_reference"
    );
}

proptest! {
    #[test]
    fn fee_and_balance_never_negative(
        tuition in 0_i64..=1_000_000,
        exam in 0_i64..=1_000_000,
        other in 0_i64..=1_000_000,
        terms in 0_u32..=12,
        transport_fee in 0_i64..=1_000_000,
        concession in 0_i64..=100_000_000,
        paid in 0_i64..=100_000_000,
    ) {
        let inputs = FeeInputs {
            plan: FeePlan { tuition, exam, other },
            terms,
            transport_fee_per_term: transport_fee,
            transport: true,
            rte: false,
            concession,
        };
        prop_assert!(session_due(&inputs) >= 0);
        prop_assert!(balance(session_due(&inputs), paid) >= 0);
    }
}

fn default_scale() -> Vec<GradeBand> {
    vec![
        GradeBand {
            grade: "A".into(),
            min_percent: 80,
        },
        GradeBand {
            grade: "B".into(),
            min_percent: 65,
        },
        GradeBand {
            grade: "C".into(),
            min_percent: 50,
        },
        GradeBand {
            grade: "D".into(),
            min_percent: 33,
        },
        GradeBand {
            grade: "E".into(),
            min_percent: 0,
        },
    ]
}

#[test]
fn marks_and_grade_vectors() {
    assert_eq!(parse_mark("", 100), Ok(MarkValue::Blank));
    assert_eq!(parse_mark("ab", 100), Ok(MarkValue::Absent));
    assert_eq!(parse_mark("AB", 100), Ok(MarkValue::Absent));
    assert_eq!(parse_mark("77", 100), Ok(MarkValue::Score(77)));
    assert!(parse_mark("101", 100).is_err());
    assert!(parse_mark("-1", 100).is_err());
    assert!(parse_mark("7.5", 100).is_err());
    let result = exam_result(
        &[MarkValue::Score(25), MarkValue::Score(26), MarkValue::Score(26)],
        42,
    );
    assert_eq!(result.got, 77);
    assert_eq!(result.max, 126);
    assert_eq!(exam_result(&[MarkValue::Score(2)], 3).percent_tenths, Some(667));
    assert_eq!(exam_result(&[MarkValue::Blank], 100).percent_tenths, None);
    let scale = default_scale();
    assert!(validate_grade_scale(&scale).is_ok());
    for (percent, grade) in [
        (800, "A"),
        (799, "B"),
        (650, "B"),
        (649, "C"),
        (616, "C"),
        (500, "C"),
        (499, "D"),
        (450, "D"),
        (330, "D"),
        (329, "E"),
    ] {
        assert_eq!(grade_for(percent, &scale), grade);
    }
    let duplicate = vec![
        GradeBand {
            grade: "A".into(),
            min_percent: 80,
        },
        GradeBand {
            grade: "A".into(),
            min_percent: 0,
        },
    ];
    assert!(validate_grade_scale(&duplicate).is_err());
    assert!(validate_grade_scale(&scale[..4]).is_err());
    let ascending = vec![
        GradeBand {
            grade: "B".into(),
            min_percent: 60,
        },
        GradeBand {
            grade: "A".into(),
            min_percent: 80,
        },
        GradeBand {
            grade: "E".into(),
            min_percent: 0,
        },
    ];
    assert!(validate_grade_scale(&ascending).is_err());
}

#[test]
fn exact_exam_vector_uses_125_maximum() {
    let values = [
        MarkValue::Score(15),
        MarkValue::Score(15),
        MarkValue::Score(15),
        MarkValue::Score(15),
        MarkValue::Score(17),
    ];
    let result = exam_result(&values, 25);
    assert_eq!(result.got, 77);
    assert_eq!(result.max, 125);
    assert_eq!(result.entered, 5);
    assert_eq!(result.percent_tenths, Some(616));
    assert_eq!(grade_for(result.percent_tenths.unwrap(), &default_scale()), "C");
}

#[test]
fn attendance_vectors() {
    assert_eq!(next_status(None), AttendanceStatus::P);
    assert_eq!(next_status(Some(AttendanceStatus::P)), AttendanceStatus::A);
    assert_eq!(next_status(Some(AttendanceStatus::A)), AttendanceStatus::L);
    assert_eq!(next_status(Some(AttendanceStatus::L)), AttendanceStatus::P);
    assert_eq!(percent(2, 3), Some(67));
    assert_eq!(percent(1, 2), Some(50));
    assert_eq!(percent(0, 0), None);
    let today = NaiveDate::from_ymd_opt(2026, 9, 17).unwrap();
    assert!(check_attendance_date(today, today, false).is_ok());
    assert_eq!(
        check_attendance_date(today.succ_opt().unwrap(), today, true)
            .unwrap_err()
            .message_key,
        "attendance.error.future"
    );
    assert_eq!(
        check_attendance_date(today.pred_opt().unwrap(), today, false)
            .unwrap_err()
            .message_key,
        "attendance.error.past_read_only"
    );
    assert!(check_attendance_date(today.pred_opt().unwrap(), today, true).is_ok());
}

#[test]
fn username_vectors() {
    assert_eq!(username_base_from_name("Sierra D'Souza"), Some("sierra".into()));
    assert_eq!(username_base_from_name("सुनीता मिश्रा"), None);
    assert_eq!(username_base_from_name("  Ravi  Kumar"), Some("ravi".into()));
    let mut taken = BTreeSet::from(["sierra".to_owned()]);
    assert_eq!(unique_username("sierra", &taken), "sierra2");
    taken.insert("sierra2".into());
    assert_eq!(unique_username("sierra", &taken), "sierra3");
    assert_eq!(fallback_username_base(Role::Teacher, 1), "teacher1");
    assert_eq!(fallback_username_base(Role::Accountant, 1), "accountant1");
}

#[test]
fn supplied_secret_bytes_are_formatted_deterministically() {
    assert_eq!(format_temp_password([0, 1, 2, 3, 4, 5, 6, 7]), "abcd-6789");
    assert_eq!(format_temp_password([23, 24, 25, 26, 8, 9, 10, 11]), "abcd-2345");
    assert_eq!(
        format_session_token([0; 32]),
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    assert_eq!(
        format_session_token([255; 32]),
        "__________________________________________8"
    );
}

#[test]
fn hlc_vectors() {
    let mut clock = HlcClock::new("device-a".into(), None);
    let first = clock.tick(1_757_923_200_000);
    let second = clock.tick(1_757_923_199_000);
    assert!(second > first);
    let remote = Hlc {
        wall_ms: 1_800_000_000_000,
        counter: 7,
        device: "device-b".into(),
    };
    let observed = clock.observe(&remote, 1_700_000_000_000);
    assert!(observed > remote);
    assert_eq!(observed.wall_ms, remote.wall_ms);
    assert_eq!(observed.counter, 8);
    let text = observed.to_text();
    assert_eq!(Hlc::parse(&text), Ok(observed));
    for invalid in [
        "",
        "1-000000-device",
        "0000000000001-1-device",
        "0000000000001-000001-",
    ] {
        assert!(Hlc::parse(invalid).is_err());
    }
}

proptest! {
    #[test]
    fn hlc_text_order_matches_tuple_order(
        left_wall in 0_u64..10_000_000_000_000,
        left_counter in 0_u32..1_000_000,
        right_wall in 0_u64..10_000_000_000_000,
        right_counter in 0_u32..1_000_000,
    ) {
        let left = Hlc { wall_ms: left_wall, counter: left_counter, device: "a".into() };
        let right = Hlc { wall_ms: right_wall, counter: right_counter, device: "b".into() };
        prop_assert_eq!(left.cmp(&right), left.to_text().cmp(&right.to_text()));
    }
}

#[test]
fn message_missing_key_returns_key() {
    assert_eq!(message(Lang::En, "not.present", &BTreeMap::new()), "not.present");
}
