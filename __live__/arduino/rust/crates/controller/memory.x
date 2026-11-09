/* Memory layout for the Arduino UNO R4 WiFi (Renesas RA4M1, R7FA4M1AB3CFM),
 * as unor4-hal's test firmware lays it out (crates/test-firmware/memory.x).
 *
 * The Arduino bootloader occupies the first 16 KiB of flash and jumps to
 * the image at 0x4000; keeping it lets arduino-cli upload over USB.
 * FLASH ORIGIN 0x00000000 would erase the bootloader.
 */
MEMORY
{
  FLASH : ORIGIN = 0x00004000, LENGTH = 240K
  RAM   : ORIGIN = 0x20000000, LENGTH = 32K
}
