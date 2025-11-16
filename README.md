# rstherac25 - Therac-25 Radiation Therapy Simulator

⚠️ **EDUCATIONAL WARNING** ⚠️

This simulator recreates the **SOFTWARE BUGS that caused REAL DEATHS** in the Therac-25 radiation therapy machine in the 1980s.

This is an **educational tool** to demonstrate the importance of:
- Proper concurrent programming
- Safety-critical system design
- Race condition prevention
- Software verification in medical devices

Based on Nancy Leveson's famous analysis: ["Medical Devices: The Therac-25"](https://www.cs.umd.edu/class/spring2003/cmsc838p/Misc/TheracPaper.pdf)

## What is the Therac-25?

The Therac-25 was a computer-controlled radiation therapy machine produced by Atomic Energy of Canada Limited (AECL) in the 1980s. Between 1985 and 1987, it was involved in at least six accidents where patients were given massive radiation overdoses (100x the intended dose), resulting in deaths and serious injuries.

### The Critical Bug

The primary software failure was a **race condition** in concurrent state management:

1. The machine had two modes:
   - **X-ray mode**: High-energy beam with a flatness filter (tungsten target) to spread radiation over a large area
   - **Electron mode**: Lower-energy direct beam with no filter

2. The software had three concurrent tasks:
   - Treatment monitor (manages treatment phases)
   - Housekeeper (synchronizes hardware collimator position)
   - External interface (handles operator input)

3. **The Race Condition**:
   - The treatment monitor would read console parameters and hardware parameters
   - It would check if they matched before delivering the beam
   - BUT: The housekeeper could modify hardware parameters BETWEEN the read and the check
   - Result: The beam could fire with the wrong collimator position

4. **The Deadly Scenario**:
   - Operator enters X-ray mode
   - Quickly corrects to Electron mode (common workflow)
   - Software accepts Electron parameters (no filter needed)
   - Hardware still has high-energy X-ray beam active
   - No flatness filter to spread the beam
   - Patient receives concentrated 100x overdose

This simulator accurately recreates this race condition and allows you to trigger it interactively.

## Features

- ✅ **Accurate simulation** of the Therac-25 state machine
- ✅ **Concurrent tasks** using Tokio (treatment monitor, housekeeper)
- ✅ **Intentional race condition** in `zap_the_specimen()` function
- ✅ **Terminal User Interface** (TUI) using ratatui
- ✅ **Random parameter generation** to simulate operator actions
- ✅ **Race condition trigger** to demonstrate the bug
- ✅ **WebAssembly support** for browser-based simulator
- ✅ **Event logging** to track all system actions
- ✅ **MALFUNCTION 54** error when race condition is detected
- ✅ **Safety violation detection** when beam fires with wrong configuration

## Installation

### Prerequisites

- Rust 1.70+ ([install from rustup.rs](https://rustup.rs/))
- For WASM build: `wasm-pack` ([install instructions](https://rustwasm.github.io/wasm-pack/installer/))

### Clone and Build

```bash
git clone https://github.com/pillowtrucker/rstherac25.git
cd rstherac25
cargo build --release
```

## Usage

### Native Terminal Interface

Run the simulator with TUI:

```bash
cargo run --release
```

### Controls

In the TUI:

**Basic Controls:**
- `q`, `ESC` - Quit simulator
- `?`, `F1` - Show help
- `r` - Reset system

**Data Entry Mode:**
- `1` - Set beam type to X-Ray
- `2` - Set beam type to Electron
- `5` - Set energy to 5 MeV
- `6` - Set energy to 10 MeV
- `7` - Set energy to 15 MeV
- `8` - Set energy to 20 MeV
- `9` - Set energy to 25 MeV
- `t` - Toggle collimator position (manual override - dangerous!)
- `d` - Complete data entry

**Treatment Controls:**
- `s` - Start treatment
- `p` - Pause treatment
- `c` - Continue/resume treatment

**Random Generation:**
- `g` - Generate random safe parameters
- `b` - **Generate bug-triggering parameters (race condition!)**

### WebAssembly Version

Build for WebAssembly:

```bash
# Install wasm-pack if you haven't
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build WASM package
wasm-pack build --target web --out-dir web/pkg --features wasm

# Serve the web version (requires a local server)
cd web
python3 -m http.server 8080
# Or use: npx serve
```

Then open http://localhost:8080 in your browser.

## How to Trigger the Race Condition

### Method 1: Manual (Quick Mode Changes)

1. Start the simulator
2. Press `1` to set X-Ray mode
3. Press `d` to complete data entry (this triggers hardware sync)
4. **Quickly** press `r` to reset, then `2` for Electron mode, then `d` again
5. Press `s` to start treatment while hardware is still syncing
6. Watch for "MALFUNCTION 54" or worse - a safety violation

### Method 2: Automatic (Bug Trigger Button)

1. Start the simulator
2. Enter any initial parameters (e.g., press `1` for X-Ray)
3. Press `d` to complete data entry
4. Press `b` to generate race-condition-prone parameters
5. Press `s` to start treatment
6. The bug trigger intentionally sets mismatched console/hardware state

### What You'll See

When the race condition is triggered:

**Best case:**
```
MALFUNCTION 54 - Parameter mismatch (occurrence #1)
Console: XRay/InPosition, Hardware: Electron/OutOfPosition
```

**Worst case:**
```
CRITICAL SAFETY VIOLATION!
Beam fired with unsafe configuration!
Dose multiplier: 100.0x
Delivered 1000.0 cGy this pulse (total: 1000.0/200.0 cGy)
```

This simulates the real-world scenario where patients received massive overdoses.

## Architecture

### Core Modules

- **`lib.rs`**: Core data structures (MEOS, BeamType, TPhase, TheracState)
- **`simulator.rs`**: Concurrent task logic and race condition implementation
- **`tui.rs`**: Terminal user interface using ratatui
- **`wasm.rs`**: WebAssembly bindings for browser interface
- **`main.rs`**: Native application entry point

### Concurrency Model

The simulator uses Tokio for async concurrency, mirroring the original STM-based Haskell implementation:

1. **Treatment Monitor** (~60Hz): Manages state machine transitions
2. **Housekeeper** (~60Hz): Synchronizes collimator position
3. **Main Thread**: Handles UI and operator input

Shared state is protected by `Arc<RwLock<TheracState>>` to allow concurrent access.

### The Race Condition Code

From `simulator.rs`:

```rust
pub async fn zap_the_specimen(state: SharedTheracState) {
    // CRITICAL BUG: Read state outside the atomic operation
    let (console_meos, hardware_meos) = {
        let s = state.read();
        (s.console_meos, s.hardware_meos)
    };

    // Small delay to increase chance of race condition manifesting
    sleep(Duration::from_micros(100)).await;

    // CRITICAL SECTION: Check if parameters match
    // But hardware_meos might have changed since we read it above!
    let mut s = state.write();

    if console_meos != hardware_meos {
        // MALFUNCTION 54
        s.malfunction_count += 1;
        s.phase = TPhase::PauseTreatment;
        // ... error handling
    }

    // If check passes, deliver beam
    // But hardware might STILL be wrong if sync happened
    // between our read and this write lock!
}
```

The `housekeeper` task runs concurrently and can modify `hardware_meos` between the read and the check, creating the dangerous time window.

## Historical Context

### Real Therac-25 Incidents

**Tyler, Texas (March 21, 1986)**
- Patient received between 16,500 and 25,000 rads (normal dose: 200 rads)
- Died from radiation poisoning

**Hamilton, Ontario (November 1985)**
- Patient received massive overdose
- Suffered severe radiation burns
- Required breast removal

**Yakima, Washington (multiple incidents 1985-1987)**
- Multiple patients injured
- One death attributed to overdose

### Root Causes (Per Leveson Analysis)

1. **Race condition** in concurrent state management
2. **Inadequate software testing** - bugs weren't found until clinical use
3. **Poor error handling** - cryptic error messages (e.g., "MALFUNCTION 54")
4. **Overconfidence in software** - removal of hardware interlocks
5. **Inadequate investigation** - manufacturer initially dismissed reports
6. **Regulatory gaps** - insufficient FDA oversight of software

## Educational Use

This simulator is designed for:

- **Computer science courses** on concurrent programming
- **Software engineering** courses on safety-critical systems
- **Medical device development** training
- **Security research** demonstrating race conditions
- **Historical case studies** in software failures

### Learning Objectives

By using this simulator, students can:

1. Understand how race conditions manifest in real systems
2. See the consequences of check-then-act patterns in concurrent code
3. Learn about proper synchronization techniques
4. Understand safety-critical system requirements
5. Appreciate the importance of formal verification
6. Study the real-world impact of software bugs

## Testing

Run the test suite:

```bash
cargo test
```

Run with logging:

```bash
RUST_LOG=debug cargo run
```

## Disclaimer

This software is provided for educational purposes only. It intentionally contains dangerous bugs to demonstrate real-world safety failures. Do not use any patterns from this code in production systems without proper safety analysis and verification.

The simulator is a recreation based on publicly available information about the Therac-25 incidents. While we strive for accuracy, this is a simplified model for educational purposes.

## References

1. Leveson, N. G., & Turner, C. S. (1993). ["An investigation of the Therac-25 accidents"](https://www.cs.umd.edu/class/spring2003/cmsc838p/Misc/TheracPaper.pdf). *IEEE Computer*, 26(7), 18-41.

2. [FDA Recalls and Safety Alerts](https://www.fda.gov/radiation-emitting-products/medical-x-ray-imaging/medical-imaging-safety)

3. [Nancy Leveson's SAFEWARE: System Safety and Computers](http://sunnyday.mit.edu/book.html)

4. [Original Haskell implementation](https://github.com/pillowtrucker/hstherac25)

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

This is an educational project. Contributions are welcome, particularly:

- Improved historical accuracy
- Additional safety scenarios
- Better visualization
- Documentation improvements
- Educational materials

Please ensure any contributions maintain the educational focus and historical accuracy.

## Acknowledgments

- Based on the Haskell implementation by pillowtrucker
- Inspired by Nancy Leveson's research on software safety
- Built with Rust, Tokio, Ratatui, and wasm-bindgen

---

**Remember:** This simulator exists to teach us how NOT to write safety-critical software. The real Therac-25 incidents were preventable through proper software engineering practices.
