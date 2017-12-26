#ifndef SCANNER_H_
#define SCANNER_H_

#include <Arduino.h>
#include "auto_config.h"
#include "VolatileFlag.h"
#include "Timer.h"


// Use input without pullup as high-impedance state
#define HI_Z INPUT


class Matrix{
public:
  Matrix();
  bool get(uint8_t index);
  void setup();

  bool scanIfChanged();
  bool isInStandby();
  bool isSquishedInBackpack();
  void shutdown();

private:

  void scan();
  void enterStandby();
  void exitStandby();


  void enablePinChangeInterrupt();
  void disablePinChangeInterrupt();

  void setRowsInput();
  void setColumnsLow();
  void setColumnsHiZ();
  void attachRowPinInterrupts(voidFuncPtr isr);
  void detachRowPinInterrupts();

  static void pinChangeISR();

  void printPressedSwitch(uint8_t c, uint8_t r);

  Timer* standby_timer;
  Timer* squished_switch_timer;
  const uint32_t squished_delay = 100000;

  bool pressed [NUM_MATRIX_POSITIONS] = {0};

};

#endif
