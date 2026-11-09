// apply_timer: the R3 on the HAL's bench as a timing instrument for gap 8 of
// real-timings.md: from the bench R4's processor having a command whole to
// the step generator running at the command's new rate
// (__workbench__/README.md, The apply test). The R4
// (__workbench__/uno_4r/crates/apply-firmware) flips D3 the moment it has a
// command; this sketch times that edge and every STEP pulse on one clock,
// this board's Timer1, and reports each command's delay as it happens.
//
// Wiring (unor4-hal Validation.md, All-in-one validation): R4 D3 <-> R3 D3,
// R4 D4 (STEP) -> R3 D8, GND <-> GND. This sketch never drives a pin.
//
// The clock: Timer1 counts the 16 MHz crystal with no prescaler, so one
// count is 62.5 ns (ATmega328P datasheet, 16.11, CS10). Its overflow
// interrupt extends the 16-bit count to 32 bits, 268 s before it wraps.
//
// STEP's edges: D8 is ICP1, Timer1's input capture. The timer's count is
// latched in hardware at each falling edge (ICES1 = 0), so a pulse's time is
// exact to the count, whatever the interrupt's delay. A falling edge is
// where the R4's step generator ends one period and starts the next
// (unor4-hal step/step_generator.rs: the output goes LOW at the cycle end),
// so the time between two falling edges is one period, and a new rate
// starts at a falling edge.
//
// D3's edges: D3 is INT1, interrupting on any change. Its time is read when
// the handler runs, a few us after the edge, or later by up to one capture
// or overflow handler that is running then. So each delay is short by those
// few us.
//
// How a delay is found: before a mark, every period is taken as the steady
// one. After a mark, the first period that differs from the steady one by
// more than change_ticks is the first at the new rate; it began at the
// falling edge before it, so the delay is that edge's time less the mark's.
// A period that never changes within timeout_us is reported as N; a change
// that began before the mark, as E.
//
// The protocol, one line each way, 115,200 8N1, '\n' ends a line:
//   at start                          -> READY apply_timer
//   start <change_ticks> <timeout_us> -> OK start <change_ticks> <timeout_us>,
//                                        then one line per mark, below
//   stop                              -> OK stop <dropped>, the results it had
//                                        no room for
//   ?                                 -> apply_timer
// One line per mark:
//   A <delay_ticks> <old_period_ticks> <new_period_ticks>   applied
//   E <ticks_before> <old_period_ticks> <new_period_ticks>  began before it
//   N 0 <old_period_ticks> 0                                no change seen

#include <Arduino.h>

// The line speed; apply-host.toml's [r3] baud must match.
static const unsigned long SERIAL_BAUD = 115200;

// Timer1 counts per us: 16 MHz with no prescaler.
static const uint8_t TICKS_PER_US = 16;

// The pins: the R4's mark on D3, which is INT1 (PD3), and the R4's STEP on
// D8, which is ICP1, Timer1's input capture (ATmega328P datasheet, 1.1, pin
// configurations; the R3's header labels).
static const uint8_t MARK_PIN = 3;
static const uint8_t STEP_PIN = 8;

// Half of the 16-bit count's range. In `widen`, a count read with an
// overflow still pending is below this only if it was read after the wrap,
// so the pending overflow belongs to it; a count at or above it was read
// before the wrap, and the overflow comes after it. Any count is read
// within a few hundred counts of its event, far less than half the range.
static const uint16_t HALF_RANGE = 0x8000;

// The longest line taken from the PC, bytes.
static const uint8_t LINE_MAX = 40;

// Results waiting to be sent: a mark comes every 33-50 ms and a line takes
// about 3 ms to send, so a few slots are plenty.
static const uint8_t RING_SIZE = 8;

// The result kinds, the first character of each line.
static const char KIND_APPLIED = 'A';
static const char KIND_EARLY = 'E';
static const char KIND_NONE = 'N';

struct Result {
  char kind;
  uint32_t ticks;
  uint32_t old_period;
  uint32_t new_period;
};

static volatile uint16_t overflows = 0;

static volatile bool timing = false;
static volatile uint32_t change_ticks = 0;
static volatile uint32_t timeout_ticks = 0;

static volatile bool have_capture = false;
static volatile uint32_t last_capture = 0;
static volatile uint32_t steady_period = 0;

static volatile bool armed = false;
static volatile uint32_t mark_at = 0;

static volatile Result ring[RING_SIZE];
static volatile uint8_t ring_head = 0;
static volatile uint8_t ring_tail = 0;
static volatile uint32_t dropped = 0;

static char line[LINE_MAX + 1];
static uint8_t line_len = 0;

ISR(TIMER1_OVF_vect) {
  overflows++;
}

// A 16-bit count of Timer1 as 32 bits. Called with interrupts off: a
// pending overflow that its handler hasn't counted yet is counted here, when
// the count has already wrapped past it.
static uint32_t widen(uint16_t low) {
  uint16_t high = overflows;
  if ((TIFR1 & _BV(TOV1)) && low < HALF_RANGE) {
    high++;
  }
  return ((uint32_t)high << 16) | low;
}

static void push(char kind, uint32_t ticks, uint32_t old_period, uint32_t new_period) {
  uint8_t next = (ring_head + 1) % RING_SIZE;
  if (next == ring_tail) {
    dropped++;
    return;
  }
  ring[ring_head].kind = kind;
  ring[ring_head].ticks = ticks;
  ring[ring_head].old_period = old_period;
  ring[ring_head].new_period = new_period;
  ring_head = next;
}

static uint32_t difference(uint32_t a, uint32_t b) {
  return a > b ? a - b : b - a;
}

ISR(TIMER1_CAPT_vect) {
  uint32_t at = widen(ICR1);
  if (have_capture) {
    uint32_t period = at - last_capture;
    if (armed) {
      if (difference(period, steady_period) > change_ticks) {
        int32_t delay = (int32_t)(last_capture - mark_at);
        if (delay >= 0) {
          push(KIND_APPLIED, (uint32_t)delay, steady_period, period);
        } else {
          push(KIND_EARLY, (uint32_t)(-delay), steady_period, period);
        }
        armed = false;
        steady_period = period;
      } else if (at - mark_at > timeout_ticks) {
        push(KIND_NONE, 0, steady_period, 0);
        armed = false;
      }
    } else {
      steady_period = period;
    }
  }
  last_capture = at;
  have_capture = true;
}

ISR(INT1_vect) {
  uint32_t at = widen(TCNT1);
  if (!timing) {
    return;
  }
  if (armed) {
    // The mark before this one never saw its new rate.
    push(KIND_NONE, 0, steady_period, 0);
  }
  armed = true;
  mark_at = at;
}

static void start(uint32_t new_change_ticks, uint32_t timeout_us) {
  cli();
  change_ticks = new_change_ticks;
  timeout_ticks = timeout_us * TICKS_PER_US;
  armed = false;
  ring_head = 0;
  ring_tail = 0;
  dropped = 0;
  // Timer0's millis() interrupt off while timing: it would delay INT1.
  TIMSK0 &= ~_BV(TOIE0);
  timing = true;
  sei();
}

static void stop() {
  cli();
  timing = false;
  armed = false;
  sei();
  TIMSK0 |= _BV(TOIE0);
}

static void handle(const char *text) {
  if (strcmp(text, "?") == 0) {
    Serial.println(F("apply_timer"));
    return;
  }
  if (strcmp(text, "stop") == 0) {
    stop();
    Serial.print(F("OK stop "));
    Serial.println(dropped);
    return;
  }
  unsigned long new_change_ticks = 0;
  unsigned long timeout_us = 0;
  if (sscanf(text, "start %lu %lu", &new_change_ticks, &timeout_us) == 2) {
    if (new_change_ticks == 0 || timeout_us == 0) {
      Serial.println(F("ERR start: change_ticks and timeout_us must be above 0"));
      return;
    }
    start(new_change_ticks, timeout_us);
    Serial.print(F("OK start "));
    Serial.print(new_change_ticks);
    Serial.print(' ');
    Serial.println(timeout_us);
    return;
  }
  Serial.println(F("ERR unknown"));
}

// Sends the results waiting, oldest first.
static void send_results() {
  while (true) {
    cli();
    if (ring_tail == ring_head) {
      sei();
      return;
    }
    Result result;
    result.kind = ring[ring_tail].kind;
    result.ticks = ring[ring_tail].ticks;
    result.old_period = ring[ring_tail].old_period;
    result.new_period = ring[ring_tail].new_period;
    ring_tail = (ring_tail + 1) % RING_SIZE;
    sei();
    Serial.print(result.kind);
    Serial.print(' ');
    Serial.print(result.ticks);
    Serial.print(' ');
    Serial.print(result.old_period);
    Serial.print(' ');
    Serial.println(result.new_period);
  }
}

void setup() {
  Serial.begin(SERIAL_BAUD);
  pinMode(MARK_PIN, INPUT);
  pinMode(STEP_PIN, INPUT);

  // Timer1 free-running: normal mode, no prescaler, capture on D8's falling
  // edge, overflow and capture interrupts on.
  TCCR1A = 0;
  TCCR1B = _BV(CS10);
  TCNT1 = 0;
  TIFR1 = _BV(TOV1) | _BV(ICF1);
  TIMSK1 = _BV(TOIE1) | _BV(ICIE1);

  // INT1 on any change of D3.
  EICRA = (EICRA & ~(_BV(ISC11) | _BV(ISC10))) | _BV(ISC10);
  EIFR = _BV(INTF1);
  EIMSK |= _BV(INT1);

  Serial.println(F("READY apply_timer"));
}

void loop() {
  send_results();
  while (Serial.available() > 0) {
    char c = (char)Serial.read();
    if (c == '\r') {
      continue;
    }
    if (c == '\n') {
      line[line_len] = '\0';
      handle(line);
      line_len = 0;
      continue;
    }
    if (line_len < LINE_MAX) {
      line[line_len++] = c;
    }
  }
}
