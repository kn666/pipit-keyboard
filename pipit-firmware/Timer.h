#ifndef TIMER_H_
#define TIMER_H_

#include "auto_config.h"
#include <Arduino.h>

class Timer{
public:
  Timer();
  Timer(uint32_t new_default_value, bool start_now);
  void start();
  void start(uint32_t new_value);
  void disable();
  bool isDisabled();
  bool isDone();
  bool peekDone();
  bool isRunning();
  void forceDone();
  void setDefaultValue(int32_t new_default_value);
  uint32_t elapsed();
  void jumpAhead(uint32_t millis_ahead);

private:
  bool is_disabled = 1;
  uint32_t start_millis = 0;
  uint32_t value = 0;
  uint32_t default_value = 0;

};





#endif