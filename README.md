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

### Operator Interface

The TUI simulates the actual Therac-25 operator terminal with form-based data entry:

**Data Entry Workflow:**

1. **Mode Entry:** Type `X` for X-ray or `E` for Electron
   - X-ray mode automatically sets energy to 25 MeV and skips to Gantry entry (as per the real Therac-25)
   - Electron mode moves you to energy entry

2. **Energy Entry:** Type energy value (5, 10, 15, 20, or 25)
   - Press ENTER without typing to copy from prescription
   - Press ENTER after typing to proceed to gantry angle

3. **Gantry Angle Entry:** Type gantry angle (0-360 degrees)
   - Press ENTER without typing to copy from prescription
   - Press ENTER after typing to proceed to field size

4. **Field Size Entry:** Type X dimension, press `x`, type Y dimension
   - Example: Type `10` then `x` then `15` for a 10×15 cm field
   - Press ENTER without typing to copy from prescription (e.g., `10x15` copied automatically)
   - Press ENTER after typing to proceed to dose

5. **Dose Entry:** Type target dose in cGy (centigray)
   - Press ENTER without typing to copy from prescription
   - Press ENTER after typing to proceed to command prompt

6. **Command Prompt:** Type a command and press ENTER
   - `t` or `treat` - Start treatment immediately
   - `r` or `reset` - Reset system and generate new prescription
   - `p` or `proceed` - Complete data entry and move to setup phase
   - `s` or `stop` - Pause active treatment
   - `c` or `continue` - Resume paused treatment
   - `q` or `quit` - Exit simulator
   - Press ESC to return to Mode entry

**Quick Entry Feature:**
- Press ENTER on any numeric field without typing to copy the prescription value
- This simulates the real operator workflow that led to the race condition
- Quick entry was convenient but dangerous when operators made mistakes!

**Field Navigation:**
- Press ENTER to advance to the next field
- Press ESC at any time to return to Mode entry
- Press Backspace to delete characters

**Global Commands:**
- `F1` - Show help screen with detailed instructions
- `Ctrl+C` - Emergency quit

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

The Therac-25 race condition typically occurred when operators:
1. Entered X-ray mode (which auto-sets high energy)
2. Noticed a mistake and quickly changed to Electron mode
3. Started treatment before the hardware sync completed
4. Result: High-energy beam fired without the flatness filter = 100x overdose

### Method 1: Realistic Operator Error (Quick Mode Change)

This simulates the exact workflow that caused real deaths:

1. **Start the simulator** - `cargo run --release`
2. **Enter X-ray mode** - Type `X` (energy auto-sets to 25 MeV, skips to Gantry)
3. **"Oops, wrong mode!"** - Press ESC to return to Mode entry
4. **Quickly change to Electron** - Type `E`
5. **Enter energy** - Type `15` and press ENTER
6. **Enter gantry** - Press ENTER to copy from prescription
7. **Enter field size** - Press ENTER to copy from prescription
8. **Enter dose** - Press ENTER to copy from prescription
9. **Start treatment immediately** - Type `t` and press ENTER
10. **Watch what happens** - The hardware may still be syncing from X-ray mode

The race window is small but real. If you start treatment while the hardware is still moving the collimator, you'll either get:
- **MALFUNCTION 54** if the mismatch is detected
- **CRITICAL SAFETY VIOLATION** if the beam fires during the sync

**Alternative Quick Trigger:**
After entering X-ray mode, quickly press ESC, type `E`, then rapidly press ENTER through all fields (copying prescription values), and type `t` to treat. This rapid-fire data entry before hardware sync completes is what caused real accidents.

### Method 2: Prescription Workflow

This demonstrates how quick-entry features can be dangerous:

1. Start the simulator (it generates a random prescription)
2. If prescription shows **X-ray**, type `X` then press ENTER on dose
3. At command prompt, type `r` to reset (generates new prescription)
4. If new prescription shows **Electron**, quickly enter parameters and treat
5. The previous X-ray configuration may still be in hardware

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
