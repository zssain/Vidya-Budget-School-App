//! seating — deterministic exam seating (P16 §10.4, Step 5).
//!
//! Pure Rust. Two classes are paired per room; their students are interleaved so
//! neighbouring seats are from different classes (no shared paper), the seats are
//! numbered **column by column**, and the result is **deterministic** for the same
//! inputs (rosters in the same order + room dimensions). A room whose capacity is
//! short returns a [`SeatingError`] listing how many seats are missing.

use serde::{Deserialize, Serialize};

/// One room's seating input: its two class rosters (ordered) and its dimensions.
/// `class_b` may be empty for a single-class room.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomPlan {
    pub room_id: String,
    pub rows: u32,
    pub cols: u32,
    pub class_a: Vec<String>,
    pub class_b: Vec<String>,
}

/// One assigned seat. `seat_no` is 1-based in column-major fill order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seat {
    pub room_id: String,
    pub seat_no: u32,
    pub student_id: String,
    /// Which paired class this seat's student belongs to ("a" or "b") — for the
    /// two-colour grid preview.
    pub class_slot: &'static str,
}

/// A room that cannot hold its paired classes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatingError {
    pub room_id: String,
    pub capacity: u32,
    pub needed: u32,
    pub missing: u32,
}

/// Generate seats for every room. Interleaves the two rosters (A, B, A, B …),
/// falling back to whichever class still has students, numbering column by column.
/// If any room is short, returns all such errors and no seats.
pub fn generate_seating(rooms: &[RoomPlan]) -> Result<Vec<Seat>, Vec<SeatingError>> {
    let mut errors = Vec::new();
    for room in rooms {
        let capacity = room.rows.saturating_mul(room.cols);
        let needed = (room.class_a.len() + room.class_b.len()) as u32;
        if needed > capacity {
            errors.push(SeatingError { room_id: room.room_id.clone(), capacity, needed, missing: needed - capacity });
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut seats = Vec::new();
    for room in rooms {
        let (mut ai, mut bi) = (0usize, 0usize);
        let total = room.class_a.len() + room.class_b.len();
        for k in 0..total {
            let prefer_a = k % 2 == 0;
            let (student, slot) = if prefer_a {
                if ai < room.class_a.len() {
                    let s = &room.class_a[ai];
                    ai += 1;
                    (s, "a")
                } else {
                    let s = &room.class_b[bi];
                    bi += 1;
                    (s, "b")
                }
            } else if bi < room.class_b.len() {
                let s = &room.class_b[bi];
                bi += 1;
                (s, "b")
            } else {
                let s = &room.class_a[ai];
                ai += 1;
                (s, "a")
            };
            seats.push(Seat { room_id: room.room_id.clone(), seat_no: (k as u32) + 1, student_id: student.clone(), class_slot: slot });
        }
    }
    Ok(seats)
}

/// Auto-pair classes into rooms two at a time, in the given order (the Principal
/// may also pair them manually — this is the default "adjacent classes" pairing).
/// Returns `(class_a, class_b)` per room index; extra classes beyond the rooms
/// are reported by the caller. Deterministic.
pub fn auto_pairs(classes: &[String]) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < classes.len() {
        let a = classes[i].clone();
        let b = classes.get(i + 1).cloned();
        out.push((a, b));
        i += 2;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(prefix: &str, n: usize) -> Vec<String> {
        (1..=n).map(|i| format!("{prefix}{i}")).collect()
    }

    #[test]
    fn interleaves_two_classes_and_is_deterministic() {
        let room = RoomPlan { room_id: "r1".into(), rows: 5, cols: 2, class_a: ids("a", 5), class_b: ids("b", 5) };
        let seats = generate_seating(std::slice::from_ref(&room)).unwrap();
        assert_eq!(seats.len(), 10);
        // Seats alternate A, B, A, B …
        assert_eq!(seats[0].student_id, "a1");
        assert_eq!(seats[1].student_id, "b1");
        assert_eq!(seats[2].student_id, "a2");
        assert_eq!(seats[3].student_id, "b2");
        // seat_no is 1..=10 in fill order.
        assert_eq!(seats.iter().map(|s| s.seat_no).collect::<Vec<_>>(), (1..=10).collect::<Vec<_>>());
        // Deterministic: same inputs → identical output.
        assert_eq!(generate_seating(&[room]).unwrap(), seats);
    }

    #[test]
    fn unequal_classes_fill_with_the_remainder() {
        // A has 4, B has 2, room holds 6.
        let room = RoomPlan { room_id: "r1".into(), rows: 3, cols: 2, class_a: ids("a", 4), class_b: ids("b", 2) };
        let seats = generate_seating(&[room]).unwrap();
        // a1,b1,a2,b2,a3,a4 (B exhausted → fill with A).
        assert_eq!(seats.iter().map(|s| s.student_id.as_str()).collect::<Vec<_>>(), vec!["a1", "b1", "a2", "b2", "a3", "a4"]);
    }

    #[test]
    fn a_short_room_reports_missing_seats() {
        // 9 students, room holds 8 → 1 missing.
        let room = RoomPlan { room_id: "r1".into(), rows: 2, cols: 4, class_a: ids("a", 5), class_b: ids("b", 4) };
        let err = generate_seating(&[room]).unwrap_err();
        assert_eq!(err, vec![SeatingError { room_id: "r1".into(), capacity: 8, needed: 9, missing: 1 }]);
    }

    #[test]
    fn single_class_room_is_allowed() {
        let room = RoomPlan { room_id: "r1".into(), rows: 2, cols: 2, class_a: ids("a", 3), class_b: vec![] };
        let seats = generate_seating(&[room]).unwrap();
        assert_eq!(seats.len(), 3);
        assert!(seats.iter().all(|s| s.class_slot == "a"));
    }

    #[test]
    fn auto_pairs_adjacent_classes() {
        assert_eq!(
            auto_pairs(&ids("c", 5)),
            vec![("c1".into(), Some("c2".into())), ("c3".into(), Some("c4".into())), ("c5".into(), None)]
        );
    }
}
