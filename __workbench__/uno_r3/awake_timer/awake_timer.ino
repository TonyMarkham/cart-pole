// awake_timer: the R3 on the HAL's bench as a timing instrument
// (__workbench__/README.md). It times every high stretch on D3, which the
// bench R4's firmware holds high while its processor is awake and low while
// it sleeps (__workbench__/uno_4r/crates/bench-firmware, sleep.rs). Every
// stretch is timed on one clock, this board's Timer1, and kept in a
// histogram, which the PC asks for at the end of a run.
//
// Wiring (unor4-hal Validation.md, All-in-one validation): R4 D3 <-> R3 D3,
// GND <-> GND. The other bench wires (D0/D1, D4 -> D8, D5 -> D7) stay as they
// are and are not used: this sketch never drives a pin.
//
// The clock: Timer1 counts the 16 MHz crystal through a prescaler of 8, so
// one count is 0.5 us (ATmega328P datasheet, 16.11, CS11). Its overflow
// interrupt extends the 16-bit count to 32 bits, 2,147 s before it wraps.
//
// How an edge is timed: D3 is INT1 (PD3), interrupting on any change. The
// handler reads Timer1 first, so the time is the edge's plus the interrupt's
// latency. Both edges of a stretch carry about the same latency, so it
// mostly cancels; what remains is up to one Timer1 overflow handler, a few
// us, when one is running as the edge comes. Timer0's interrupt (millis())
// is switched off while timing, so it adds nothing.
//
// The protocol, one line each way, 115,200 8N1, '\n' ends a line:
//   at start            -> READY awake_timer
//   start <bin_us> <bins> -> OK start <bin_us> <bins>, or ERR <why>
//   report              -> the lines below, then end
//   ?                   -> awake_timer
// The report:
//   tick_ns 500         one count of the clock, ns
//   count <n>           high stretches timed
//   missed <m>          edges whose level matched the one before: a stretch
//                       too short for the handler to see both of its edges
//   over <k>            stretches longer than bins x bin_us
//   min_ticks <x>       the shortest stretch, counts
//   max_ticks <y>       the longest stretch, counts
//   sum_ticks <s>       all stretches together, counts
//   span_ticks <t>      from start to report, counts
//   bin_us <b>          each bin's width, us
//   bin <i> <count>     one line per bin that holds any stretch: bin i holds
//                       stretches from i x bin_us up to (i + 1) x bin_us
//   end

#include <Arduino.h>

// The line speed; bench-host.toml's [r3] baud must match.
static const unsigned long SERIAL_BAUD = 115200;

// The most bins the histogram has room for: 4-byte counts, 1,000 bytes of
// the R3's 2,048 bytes of RAM. Why 4 bytes: 3,000 datagrams at the 41.5 ms
// mean gap last about 125 s, which is about 62,000 tick wakes of the R4 and
// 3,000 datagram wakes, 65,000 in all. That is already at the edge of a
// 2-byte count's 65,535, and most of them land in one or two bins, so any
// longer run would overflow one.
static const uint16_t MAX_BINS = 250;

// The widest bin `start` takes, us: MAX_BINS of it cover 0.25 s, far beyond
// any stretch awake.
static const uint16_t MAX_BIN_US = 1000;

// One Timer1 count, ns: 16 MHz / 8.
static const uint16_t TICK_NS = 500;
static const uint8_t TICKS_PER_US = 2;

// The R4's mark on D3, which is INT1 (PD3) (ATmega328P datasheet, 1.1, pin
// configurations; the R3's header labels).
static const uint8_t MARK_PIN = 3;

// Half of the 16-bit count's range. In `now_ticks`, a count read with an
// overflow still pending is below this only if it was read after the wrap,
// so the pending overflow belongs to it; a count at or above it was read
// before the wrap, and the overflow comes after it. The count is read within
// a few counts of the pending check, far less than half the range.
static const uint16_t HALF_RANGE = 0x8000;

// The longest line taken from the PC, bytes.
static const uint8_t LINE_MAX = 40;

static volatile uint16_t overflows = 0;

static volatile bool timing = false;
static volatile bool last_high = false;
static volatile bool have_rise = false;
static volatile uint32_t rise_at = 0;

static volatile uint32_t count = 0;
static volatile uint32_t missed = 0;
static volatile uint32_t over = 0;
static volatile uint32_t min_ticks = 0xFFFFFFFF;
static volatile uint32_t max_ticks = 0;
static volatile uint32_t sum_ticks = 0;
static volatile uint32_t bins_hit[MAX_BINS];

static uint16_t bin_ticks = 0;
static uint16_t bins = 0;
static uint16_t bin_us = 0;
static uint32_t started_at = 0;

static char line[LINE_MAX + 1];
static uint8_t line_len = 0;

ISR(TIMER1_OVF_vect) {
  overflows++;
}

// Timer1 as 32 bits. Called with interrupts off: a pending overflow that
// its handler hasn't counted yet is counted here, when the 16-bit count has
// already wrapped past it.
static uint32_t now_ticks() {
  uint16_t low = TCNT1;
  uint16_t high = overflows;
  if ((TIFR1 & _BV(TOV1)) && low < HALF_RANGE) {
    high++;
  }
  return ((uint32_t)high << 16) | low;
}

ISR(INT1_vect) {
  uint32_t at = now_ticks();
  bool high = (PIND & _BV(PD3)) != 0;
  if (!timing) {
    last_high = high;
    return;
  }
  if (high == last_high) {
    missed++;
    // A missed pair leaves the stretch's start unknown.
    have_rise = high;
    rise_at = at;
    return;
  }
  last_high = high;
  if (high) {
    have_rise = true;
    rise_at = at;
    return;
  }
  if (!have_rise) {
    return;
  }
  have_rise = false;
  uint32_t ticks = at - rise_at;
  count++;
  sum_ticks += ticks;
  if (ticks < min_ticks) {
    min_ticks = ticks;
  }
  if (ticks > max_ticks) {
    max_ticks = ticks;
  }
  uint32_t bin = ticks / bin_ticks;
  if (bin < bins) {
    bins_hit[bin]++;
  } else {
    over++;
  }
}

static void start(uint16_t new_bin_us, uint16_t new_bins) {
  cli();
  timing = false;
  bin_us = new_bin_us;
  bin_ticks = new_bin_us * TICKS_PER_US;
  bins = new_bins;
  for (uint16_t i = 0; i < MAX_BINS; i++) {
    bins_hit[i] = 0;
  }
  count = 0;
  missed = 0;
  over = 0;
  min_ticks = 0xFFFFFFFF;
  max_ticks = 0;
  sum_ticks = 0;
  have_rise = false;
  last_high = (PIND & _BV(PD3)) != 0;
  started_at = now_ticks();
  // Timer0's millis() interrupt off while timing: it would delay INT1.
  TIMSK0 &= ~_BV(TOIE0);
  timing = true;
  sei();
}

static void report() {
  cli();
  timing = false;
  uint32_t span = now_ticks() - started_at;
  sei();
  TIMSK0 |= _BV(TOIE0);

  Serial.print(F("tick_ns "));
  Serial.println(TICK_NS);
  Serial.print(F("count "));
  Serial.println(count);
  Serial.print(F("missed "));
  Serial.println(missed);
  Serial.print(F("over "));
  Serial.println(over);
  Serial.print(F("min_ticks "));
  Serial.println(count == 0 ? 0 : min_ticks);
  Serial.print(F("max_ticks "));
  Serial.println(max_ticks);
  Serial.print(F("sum_ticks "));
  Serial.println(sum_ticks);
  Serial.print(F("span_ticks "));
  Serial.println(span);
  Serial.print(F("bin_us "));
  Serial.println(bin_us);
  for (uint16_t i = 0; i < bins; i++) {
    if (bins_hit[i] != 0) {
      Serial.print(F("bin "));
      Serial.print(i);
      Serial.print(' ');
      Serial.println(bins_hit[i]);
    }
  }
  Serial.println(F("end"));
}

// One whole line from the PC.
static void handle(const char *text) {
  if (strcmp(text, "?") == 0) {
    Serial.println(F("awake_timer"));
    return;
  }
  if (strcmp(text, "report") == 0) {
    if (bins == 0) {
      Serial.println(F("ERR no start"));
      return;
    }
    report();
    return;
  }
  unsigned int new_bin_us = 0;
  unsigned int new_bins = 0;
  if (sscanf(text, "start %u %u", &new_bin_us, &new_bins) == 2) {
    if (new_bin_us == 0 || new_bin_us > MAX_BIN_US || new_bins == 0 || new_bins > MAX_BINS) {
      Serial.print(F("ERR start: bin_us 1-"));
      Serial.print(MAX_BIN_US);
      Serial.print(F(", bins 1-"));
      Serial.println(MAX_BINS);
      return;
    }
    start(new_bin_us, new_bins);
    Serial.print(F("OK start "));
    Serial.print(new_bin_us);
    Serial.print(' ');
    Serial.println(new_bins);
    return;
  }
  Serial.println(F("ERR unknown"));
}

void setup() {
  Serial.begin(SERIAL_BAUD);
  pinMode(MARK_PIN, INPUT);

  // Timer1 free-running: normal mode, prescaler 8, overflow interrupt on.
  TCCR1A = 0;
  TCCR1B = _BV(CS11);
  TCNT1 = 0;
  TIFR1 = _BV(TOV1);
  TIMSK1 = _BV(TOIE1);

  // INT1 on any change of D3.
  EICRA = (EICRA & ~(_BV(ISC11) | _BV(ISC10))) | _BV(ISC10);
  EIFR = _BV(INTF1);
  EIMSK |= _BV(INT1);

  Serial.println(F("READY awake_timer"));
}

void loop() {
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
