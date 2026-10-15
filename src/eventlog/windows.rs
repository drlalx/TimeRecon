use chrono::{DateTime, Local};
use std::iter::once;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_ITEMS, GetLastError};
use windows::Win32::System::EventLog::{
    EVT_HANDLE, EvtClose, EvtNext, EvtQuery, EvtQueryChannelPath, EvtRender, EvtRenderEventXml,
};
use windows::core::PCWSTR;

use super::{EventEntry, EventKind, SegmentKind};

const KERNEL_POWER: &str = "Microsoft-Windows-Kernel-Power";
const POWER_TROUBLESHOOTER: &str = "Microsoft-Windows-Power-Troubleshooter";

/// Queries the Windows Event Log API for session and power events.
pub(crate) fn query_event_log(days: i64) -> Vec<EventEntry> {
    let query = build_event_query(days);
    let channel = wide_null("System");
    let query_wide = wide_null(&query);
    let result_set = unsafe {
        EvtQuery(
            None,
            PCWSTR(channel.as_ptr()),
            PCWSTR(query_wide.as_ptr()),
            EvtQueryChannelPath.0,
        )
    };

    let result_set = match result_set {
        Ok(handle) => EventHandle(handle),
        Err(e) => {
            eprintln!("Failed to query event log: {e}");
            return Vec::new();
        }
    };

    let mut entries = Vec::new();
    let mut handles = [0isize; 16];

    loop {
        let mut returned = 0;
        let next = unsafe { EvtNext(result_set.0, &mut handles, 0, 0, &mut returned) };

        if let Err(error) = next {
            if unsafe { GetLastError() } != ERROR_NO_MORE_ITEMS {
                eprintln!("Failed to enumerate event log: {error}");
            }
            break;
        }

        for handle in handles.iter().take(returned as usize).copied() {
            let event = EventHandle(EVT_HANDLE(handle));
            if let Some(xml) = render_event_xml(event.0) {
                if let Some(entry) = parse_event_xml(&xml) {
                    entries.push(entry);
                }
            }
        }
    }

    entries.sort_by_key(|entry| entry.time);
    entries
}

fn build_event_query(days: i64) -> String {
    let milliseconds = days.max(0).saturating_mul(86_400_000);
    format!(
        "*[System[TimeCreated[timediff(@SystemTime) <= {milliseconds}] and (EventID=6005 or EventID=6006 \
         or (Provider[@Name='{KERNEL_POWER}'] and (EventID=42 or EventID=107 or EventID=506 or EventID=507)) \
         or (Provider[@Name='{POWER_TROUBLESHOOTER}'] and EventID=1))]]"
    )
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(once(0)).collect()
}

fn render_event_xml(event: EVT_HANDLE) -> Option<String> {
    let mut buffer_used: u32 = 0;
    let mut property_count: u32 = 0;
    let buffer_size = unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0,
            0,
            None,
            &mut buffer_used,
            &mut property_count,
        )
    };

    match buffer_size {
        Err(_) if unsafe { GetLastError() } == ERROR_INSUFFICIENT_BUFFER => {}
        Err(error) => {
            eprintln!("Failed to determine event XML buffer size: {error}");
            return None;
        }
        Ok(()) => {
            eprintln!("Event XML buffer size was unexpectedly zero.");
            return None;
        }
    }

    let mut buffer = vec![0u16; buffer_used.div_ceil(2) as usize];
    let rendered = unsafe {
        EvtRender(
            None,
            event,
            EvtRenderEventXml.0,
            buffer_used,
            Some(buffer.as_mut_ptr().cast()),
            &mut buffer_used,
            &mut property_count,
        )
    };
    if let Err(error) = rendered {
        eprintln!("Failed to render event XML: {error}");
        return None;
    }

    String::from_utf16(&buffer[..buffer_used as usize / 2])
        .ok()
        .map(|xml| xml.trim_end_matches('\0').to_owned())
}

fn parse_event_xml(xml: &str) -> Option<EventEntry> {
    let event_id: u32 = element_text(xml, "EventID")?.parse().ok()?;
    let provider = attribute_value(xml, "Provider", "Name").unwrap_or_default();
    let time = parse_time(attribute_value(xml, "TimeCreated", "SystemTime")?)?;

    let kind = match (provider, event_id) {
        (_, 6005) => EventKind::Boot,
        (_, 6006) => EventKind::Shutdown,
        (KERNEL_POWER, 42) => {
            // SYSTEM_POWER_STATE: 4 = S3 sleep, 5 = S4 hibernate, 6 = shutdown.
            let hibernate = data_value(xml, "TargetState")
                .and_then(|state| state.parse::<u32>().ok())
                .is_some_and(|state| state >= 5);
            EventKind::EnteringSleep(if hibernate {
                SegmentKind::Hibernate
            } else {
                SegmentKind::Sleep
            })
        }
        (KERNEL_POWER, 107) => EventKind::Resumed,
        (KERNEL_POWER, 506) => EventKind::EnteringStandby,
        (KERNEL_POWER, 507) => EventKind::ExitingStandby,
        (POWER_TROUBLESHOOTER, 1) => EventKind::ResumedFromLowPower {
            sleep: parse_time(data_value(xml, "SleepTime")?)?,
            wake: parse_time(data_value(xml, "WakeTime")?)?,
        },
        _ => return None,
    };

    Some(EventEntry { time, kind })
}

fn parse_time(value: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|time| time.with_timezone(&Local))
}

fn data_value<'a>(xml: &'a str, name: &str) -> Option<&'a str> {
    let (_, rest) = xml
        .split_once(&format!("Name='{name}'"))
        .or_else(|| xml.split_once(&format!("Name=\"{name}\"")))?;
    let (_, text) = rest.split_once('>')?;
    let (text, _) = text.split_once('<')?;
    Some(text.trim())
}

fn element_text<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let (_, element) = xml.split_once(&format!("<{tag}"))?;
    let (_, text) = element.split_once('>')?;
    let (text, _) = text.split_once(&format!("</{tag}>"))?;
    Some(text.trim())
}

fn attribute_value<'a>(xml: &'a str, element: &str, attribute: &str) -> Option<&'a str> {
    let (_, element) = xml.split_once(&format!("<{element}"))?;
    let (_, value) = element.split_once(&format!("{attribute}="))?;
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let (value, _) = value[quote.len_utf8()..].split_once(quote)?;
    Some(value)
}

struct EventHandle(EVT_HANDLE);

impl Drop for EventHandle {
    fn drop(&mut self) {
        let _ = unsafe { EvtClose(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_xpath_query_for_requested_time_range() {
        assert_eq!(
            build_event_query(2),
            "*[System[TimeCreated[timediff(@SystemTime) <= 172800000] and (EventID=6005 or EventID=6006 \
             or (Provider[@Name='Microsoft-Windows-Kernel-Power'] and (EventID=42 or EventID=107 or EventID=506 or EventID=507)) \
             or (Provider[@Name='Microsoft-Windows-Power-Troubleshooter'] and EventID=1))]]"
        );
    }

    #[test]
    fn parses_hibernate_entry_from_kernel_power_event() {
        let xml = "<Event><System><Provider Name='Microsoft-Windows-Kernel-Power' Guid='{x}'/>\
                   <EventID>42</EventID><TimeCreated SystemTime='2026-09-24T10:00:00.0000000Z'/></System>\
                   <EventData><Data Name='TargetState'>5</Data></EventData></Event>";

        let entry = parse_event_xml(xml).expect("parsed event");

        assert_eq!(entry.kind, EventKind::EnteringSleep(SegmentKind::Hibernate));
    }

    #[test]
    fn parses_sleep_and_wake_time_from_power_troubleshooter_event() {
        let xml = "<Event><System><Provider Name='Microsoft-Windows-Power-Troubleshooter'/>\
                   <EventID>1</EventID><TimeCreated SystemTime='2026-09-24T11:00:05.0000000Z'/></System>\
                   <EventData><Data Name='SleepTime'>2026-09-24T10:00:00.1234567Z</Data>\
                   <Data Name='WakeTime'>2026-09-24T11:00:00.1234567Z</Data></EventData></Event>";

        let entry = parse_event_xml(xml).expect("parsed event");

        let EventKind::ResumedFromLowPower { sleep, wake } = entry.kind else {
            panic!("unexpected kind {:?}", entry.kind);
        };
        assert_eq!((wake - sleep).num_minutes(), 60);
    }

    #[test]
    fn ignores_event_id_1_from_other_providers() {
        let xml = "<Event><System><Provider Name='Microsoft-Windows-Kernel-General'/>\
                   <EventID>1</EventID><TimeCreated SystemTime='2026-09-24T11:00:05.0000000Z'/></System></Event>";

        assert!(parse_event_xml(xml).is_none());
    }
}
