

## Auto-generated signatures
<!-- Updated by gen-context.js -->
You are a coding assistant with complete knowledge of this codebase.
The following code signatures were extracted by SigMap v8.28.1 on 2026-10-15T00:39:35.839Z.
<!-- sigmap: version=8.28.1 -->

These signatures represent every public function, class, and type in the project.
Refer to them when answering questions about code structure, APIs, and implementation.
## Code Signatures

## SigMap commands

| When | Command |
|------|---------|
| Before answering a question about code | `sigmap ask "<your question>"` |
| To rank files by topic | `sigmap --query "<topic>"` |
| After changing config or source dirs | `sigmap validate` |
| To verify an AI answer is grounded | `sigmap judge --response <file>` |

Always run `sigmap ask` (or `sigmap --query`) before searching for files relevant to a task.

## src

### src/egui_ui.rs
```
impl ActiveTimeApp  :108-157
impl ActiveTimeApp  :159-185
impl ActiveTimeApp  :187-314
impl Dashboard  :357-479
pub fn run() → eframe::Result  :31-47
```

### src/eventlog/mod.rs
```
pub struct PcSession  :33-36  # A single PC session (boot → shutdown)
pub struct Segment  :57-61  # A continuous time range in which the PC was in one [`Segment
pub struct DaySummary  :71-79  # Daily summary of PC active time
pub enum SegmentKind  :48-53  # State of the PC during a [`Segment`]; ordered by how "off" t
impl PcSession  :38-44
  pub fn duration_secs(&self) → i64  :40-40  # Duration in seconds
impl Segment  :63-67
  pub fn duration_secs(&self) → i64  :64-64  # Duration in seconds
pub fn read_pc_sessions(days: i64) → Vec<PcSession>  :83-86  # Reads PC start and shutdown events
pub fn daily_summaries(days: i64) → Vec<DaySummary>  :89-94  # Returns daily summaries for the last N days, newest first
pub fn export_csv(days: i64) → String  :300-322  # Generates CSV export string
```

### src/eventlog/windows.rs
```
impl EventHandle  :193-197
```

### src/tui/app.rs
```
pub struct TuiApp  :33-40  # State of the TUI application
pub enum Tab  :11-14  # Active tab in the TUI
impl Tab  :16-30
impl TuiApp  :42-126
  pub fn new() → Self  :44-44  # Loads session and summary data from event logs
  pub fn refresh(&mut self)  :58-58  # Re-reads system logs and updates summaries and sessions
  pub fn export_csv(&mut self)  :67-67  # Exports CSV data to the user's Documents directory
  pub fn next_day(&mut self)  :82-82
  pub fn prev_day(&mut self)  :88-88
  pub fn next_tab(&mut self)  :92-92
  pub fn is_pc_on(&self) → bool  :96-96
  pub fn today_work_seconds(&self) → i64  :100-100
```

### src/tui/mod.rs
```
impl TerminalGuard  :21-26
pub fn run() → io::Result<()>  :29-85  # Runs the interactive Terminal User Interface
```

### src/tui/timeline.rs
```
pub enum CellKind  :11-20  # The state of a single cell in the 24-hour terminal timeline
impl CellKind  :22-33
pub fn render_timeline_cells(day: &DaySummary, width: usize) → Vec<CellKind>  :39-86  # Discretizes a [`DaySummary`] into `width` timeline cells
pub fn render_timeline_string(day: &DaySummary, width: usize) → String  :90-95  # Converts a [`DaySummary`] directly into a timeline string of
pub fn format_duration(seconds: i64) → String  :98-103  # Formats duration in seconds into `Xh YYm`
pub fn format_day_span(day: &DaySummary) → String  :106-130  # Returns the formatted time span `HH:MM → HH:MM` (or `now`) f
```

### src/tui/view.rs
```
pub fn draw(frame: &mut Frame, app: &TuiApp)  :13-33  # Draws the complete TUI interface on the frame
```
