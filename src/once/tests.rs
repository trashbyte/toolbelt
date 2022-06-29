#![allow(non_snake_case)]

use super::*;

fn increment(x: &mut u32) {
    *x += 1;
}

#[test]
fn DoOnce_only_executes_once() {
    let mut task = DoOnce::new();
    let mut x = 1;
    task.do_once(|| increment(&mut x));
    task.do_once(|| increment(&mut x));
    task.do_once(|| increment(&mut x));
    assert_eq!(x, 2);
}

#[test]
fn DoOnceSync_only_executes_once() {
    let task = DoOnceSync::new();
    let mut x = 1;
    task.do_once(|| increment(&mut x));
    task.do_once(|| increment(&mut x));
    task.do_once(|| increment(&mut x));
    assert_eq!(x, 2);
}

#[test]
fn DoOnce_done() {
    let mut task = DoOnce::new();
    assert_eq!(task.done(), false);
    task.do_once(||{});
    assert_eq!(task.done(), true);
}

#[test]
#[should_panic]
fn InitOnce_uninitialized_get_should_panic() {
    let cell: InitOnce<u32> = InitOnce::uninitialized();
    cell.get();
}

#[test]
fn InitOnce_uninitialized_try_get_returns_None() {
    let cell: InitOnce<u32> = InitOnce::uninitialized();
    assert!(cell.try_get().is_none());
}

#[test]
fn InitOnce_initialize_then_get_and_try_get() {
    let cell: InitOnce<u32> = InitOnce::uninitialized();
    assert!(cell.try_get().is_none());
    cell.initialize(1).unwrap();
    assert_eq!(cell.try_get(), Some(&1));
    assert_eq!(cell.get(), &1);
}

#[test]
fn InitOnce_get_or_init_then_get_and_try_get() {
    let cell: InitOnce<u32> = InitOnce::uninitialized();
    assert!(cell.try_get().is_none());
    assert_eq!(cell.get_or_init(|| 1).unwrap(), &1);
    assert_eq!(cell.get_or_init(|| 1).unwrap(), &1);
    assert_eq!(cell.try_get(), Some(&1));
    assert_eq!(cell.get(), &1);
}

#[test]
fn InitOnce_get_or_init_only_executes_once() {
    let cell: InitOnce<u32> = InitOnce::uninitialized();
    let mut x = 1;
    assert_eq!(cell.get_or_init(|| { x += 1; x }).unwrap(), &2);
    assert_eq!(cell.get_or_init(|| { x += 1; x }).unwrap(), &2);
    assert_eq!(cell.get_or_init(|| { x += 1; x }).unwrap(), &2);
    assert_eq!(cell.try_get(), Some(&2));
    assert_eq!(cell.get(), &2);
}

#[test]
#[should_panic]
fn InitOnce_reentrant_init_should_panic() {
    let cell: InitOnce<u32> = InitOnce::uninitialized();
    cell.get_or_init(|| { cell.initialize(1).unwrap(); 1 }).unwrap();
}
