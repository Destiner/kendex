//! Spreading one pure function over the machine's cores.
//!
//! Scoring installed content is the slowest thing kendex does, and every
//! item is scored independently of every other — so the work is a fan-out
//! with nothing shared. What this must not become is a source of
//! nondeterminism: results come back in the order the inputs were given,
//! never the order the threads happened to finish, so two runs over the
//! same disk produce byte-identical output.
//!
//! Work is taken one item at a time rather than dealt out in equal slices.
//! Items are wildly uneven — a hook is a line of shell, a skill is a tree
//! of documents — and a fixed split leaves one lane grinding while the rest
//! stand idle.

use std::sync::atomic::{AtomicUsize, Ordering};

/// How many lanes are worth opening. Scoring is memory-bound more than it
/// is compute-bound: on a 32-core machine, 32 lanes finish a large project
/// in 76 ms and spend a second of CPU doing it, while 8 lanes take 113 ms
/// for two thirds of the CPU. Neither is a wait a person notices, and an
/// audit is background work — so the machine keeps the rest of itself.
const LANE_CAP: usize = 8;

/// Apply `work` to every item, on as many cores as the machine offers.
/// A panic inside `work` is re-raised here, exactly as it would surface if
/// this ran on one thread.
pub fn map<T, R>(items: &[T], work: impl Fn(&T) -> R + Sync) -> Vec<R>
where
    T: Sync,
    R: Send,
{
    let lanes = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(items.len())
        .min(LANE_CAP);
    if lanes <= 1 {
        return items.iter().map(work).collect();
    }

    let next = AtomicUsize::new(0);
    let work = &work;
    let parts: Vec<Vec<(usize, R)>> = std::thread::scope(|scope| {
        let lanes: Vec<_> = (0..lanes)
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(at) else { break };
                        mine.push((at, work(item)));
                    }
                    mine
                })
            })
            .collect();
        lanes
            .into_iter()
            .map(|lane| match lane.join() {
                Ok(done) => done,
                Err(panic) => std::panic::resume_unwind(panic),
            })
            .collect()
    });

    let mut done: Vec<(usize, R)> = parts.into_iter().flatten().collect();
    done.sort_by_key(|(at, _)| *at);
    done.into_iter().map(|(_, result)| result).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_come_back_in_the_order_the_inputs_were_given() {
        let items: Vec<usize> = (0..500).collect();
        // Uneven work, so lanes finish out of order if they are going to.
        let doubled = map(&items, |n| {
            let mut sum = 0;
            for step in 0..(n % 7) * 1000 {
                sum += step;
            }
            (n * 2, sum)
        });
        assert_eq!(doubled.len(), 500);
        for (at, (value, _)) in doubled.iter().enumerate() {
            assert_eq!(*value, at * 2);
        }
    }

    #[test]
    fn an_empty_list_needs_no_threads() {
        let nothing: Vec<usize> = Vec::new();
        assert!(map(&nothing, |n| *n).is_empty());
    }

    #[test]
    fn a_panic_in_the_work_surfaces_here() {
        let items: Vec<usize> = (0..64).collect();
        let panicked = std::panic::catch_unwind(|| {
            map(&items, |n| {
                assert_ne!(*n, 40, "boom");
                *n
            })
        });
        assert!(panicked.is_err());
    }
}
