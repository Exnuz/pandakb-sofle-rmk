MEMORY
{
  /* Preserve the MBR, storage partition, and Adafruit bootloader. */
  FLASH : ORIGIN = 0x00001000, LENGTH = 892K
  RAM : ORIGIN = 0x20000008, LENGTH = 255K
}
