use crate::{geometry::point::Point, pool::Pool};

fn ages(pool: &Pool) -> Vec<f64> {
    pool.entries().map(|entry| entry.age_ms).collect()
}

#[test]
fn given_the_pre_warmed_pool_when_read_then_six_entries_have_no_detection_one_gap_apart() {
    // Given
    let pool = Pool::pre_warmed(40.0);

    // When
    let entries: Vec<_> = pool.entries().collect();

    // Then
    assert_eq!(entries.len(), 6);
    assert_eq!(
        ages(&pool),
        vec![-40.0, -80.0, -120.0, -160.0, -200.0, -240.0]
    );
    assert!(
        entries
            .iter()
            .all(|entry| entry.pivot == Point::NOT_DETECTED && entry.end == Point::NOT_DETECTED)
    );
}

#[test]
fn given_the_pre_warmed_pool_when_the_first_frame_is_pushed_then_its_age_is_0_and_nothing_else_ages()
 {
    // Given
    let mut pool = Pool::pre_warmed(40.0);

    // When
    pool.push(
        5_000_000_000,
        Point { x: 0.0, z: 0.082 },
        Point { x: 0.0, z: -0.368 },
    );

    // Then: the synthetic Orin's frame 0: ages 0, -d1, ..., -(d1+...+d5)
    assert_eq!(ages(&pool), vec![0.0, -40.0, -80.0, -120.0, -160.0, -200.0]);
    assert_eq!(
        pool.entries().next().unwrap().pivot,
        Point { x: 0.0, z: 0.082 }
    );
}

#[test]
fn given_a_frame_when_the_next_is_pushed_then_every_older_entry_ages_by_the_stamps_difference() {
    // Given: a frame, then one 33.3 ms later by GStreamer's stamps
    let mut pool = Pool::pre_warmed(40.0);
    pool.push(
        5_000_000_000,
        Point { x: 0.0, z: 0.082 },
        Point { x: 0.0, z: -0.368 },
    );

    // When
    pool.push(
        5_033_300_000,
        Point { x: 0.01, z: 0.082 },
        Point { x: 0.02, z: -0.367 },
    );

    // Then
    let ages = ages(&pool);
    let expected = [0.0, -33.3, -73.3, -113.3, -153.3, -193.3];
    for (age, expected) in ages.iter().zip(expected) {
        assert!((age - expected).abs() < 1e-9, "{ages:?}");
    }
    assert_eq!(
        pool.entries().next().unwrap().end,
        Point { x: 0.02, z: -0.367 }
    );
}

#[test]
fn given_seven_frames_when_pushed_then_the_oldest_drops_and_the_newest_is_first() {
    // Given
    let mut pool = Pool::pre_warmed(40.0);

    // When
    for k in 0..7u64 {
        let x = k as f64 / 100.0;
        pool.push(k * 16_700_000, Point { x, z: 0.082 }, Point::NOT_DETECTED);
    }

    // Then
    let xs: Vec<f64> = pool.entries().map(|entry| entry.pivot.x).collect();
    assert_eq!(xs, vec![0.06, 0.05, 0.04, 0.03, 0.02, 0.01]);
    assert!((ages(&pool)[5] - -83.5).abs() < 1e-9);
}

#[test]
fn given_a_pool_when_flattened_then_each_entry_is_its_age_then_pivot_then_end() {
    // Given
    let mut pool = Pool::pre_warmed(40.0);
    pool.push(0, Point { x: 0.1, z: 0.2 }, Point { x: 0.3, z: 0.4 });

    // When
    let flat = pool.flat();

    // Then
    assert_eq!(flat.len(), 30);
    assert_eq!(&flat[..5], &[0.0, 0.1, 0.2, 0.3, 0.4]);
    assert_eq!(&flat[5..10], &[-40.0, -1.0, -1.0, -1.0, -1.0]);
}
