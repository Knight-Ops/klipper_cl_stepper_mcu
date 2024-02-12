use core::cell::RefCell;

use embassy_time::Instant;
use heapless::Entry::{Occupied, Vacant};
use rtos_trace::RtosTrace;
rtos_trace::global_trace!(LogRtosTracer);

use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex};

pub struct LogRtosTracer;

static ACTIVE_TASK: Mutex<CriticalSectionRawMutex, RefCell<u32>> = Mutex::new(RefCell::new(0));
static TASK_TIMER: Mutex<CriticalSectionRawMutex, RefCell<embassy_time::Instant>> =
    Mutex::new(RefCell::new(Instant::from_ticks(0)));

static TASK_TRACKING: Mutex<CriticalSectionRawMutex, RefCell<heapless::FnvIndexMap<u32, u64, 16>>> =
    Mutex::new(RefCell::new(heapless::FnvIndexMap::new()));

static SCHEDULE_TRACKER: Mutex<CriticalSectionRawMutex, RefCell<u32>> = Mutex::new(RefCell::new(0));

impl RtosTrace for LogRtosTracer {
    fn task_new(id: u32) {
        log::info!("task_new : {id}")
    }
    fn isr_enter() {
        log::info!("ISR enter");
    }
    fn isr_exit() {
        log::info!("ISR exit");
    }
    fn isr_exit_to_scheduler() {
        log::info!("ISR exit to scheduler");
    }
    fn marker(id: u32) {
        log::info!("Marker : {id}");
    }
    fn marker_begin(id: u32) {
        log::info!("Marker start : {id}");
    }
    fn marker_end(id: u32) {
        log::info!("Marker end : {id}");
    }
    fn system_idle() {
        // log::info!("Start System Idle");
    }
    fn task_exec_begin(id: u32) {
        ACTIVE_TASK.lock(|unlocked| {
            let mut task = unlocked.borrow_mut();
            *task = id;
        });
        TASK_TIMER.lock(|unlocked| {
            let mut instant = unlocked.borrow_mut();
            *instant = Instant::now()
        });
    }
    fn task_exec_end() {
        let elapsed = TASK_TIMER
            .lock(|unlocked| {
                let instant = unlocked.borrow();
                *instant
            })
            .elapsed()
            .as_micros();

        let task = ACTIVE_TASK.lock(|unlocked| {
            let task = unlocked.borrow();
            *task
        });

        TASK_TRACKING.lock(|unlocked| {
            let mut map = unlocked.borrow_mut();

            match map.entry(task) {
                Occupied(mut e) => {
                    let cur_val = e.get_mut();
                    *cur_val = *cur_val + elapsed;
                }
                Vacant(e) => {
                    e.insert(elapsed).unwrap();
                }
            }
        });

        let count = SCHEDULE_TRACKER.lock(|unlocked| {
            let mut count = unlocked.borrow_mut();
            *count = count.wrapping_add(1);
            *count
        });

        if count % 10000 == 0 {
            TASK_TRACKING.lock(|unlocked| {
                let map = unlocked.borrow();
                log::info!("Task breakdown : {:#?}", *map)
            });
        }
    }
    fn task_ready_begin(id: u32) {
        // log::info!("Task ready begin : {id}");
    }
    fn task_ready_end(id: u32) {
        log::info!("Task ready end : {id}");
    }
    fn task_send_info(id: u32, info: rtos_trace::TaskInfo) {
        log::info!(
            "Task send info : {id} - {}, {}, {}, {}",
            info.name,
            info.priority,
            info.stack_base,
            info.stack_size
        );
    }
    fn task_terminate(id: u32) {
        log::info!("Task terminate : {id}");
    }
}
