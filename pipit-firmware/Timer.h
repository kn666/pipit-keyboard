#pragma once

#include <stdint.h>
#include "auto_config.h"

class Timer {
 public:
  Timer();
  Timer(uint32_t new_default_value);
  void setDefaultValue(int32_t new_default_value);
  void start();
  void start(uint32_t new_value);
  void disable();
  bool isDisabled();
  bool isDone();
  bool peekDone();
  bool isRunning();
  void forceDone();
  uint32_t elapsed();
  uint32_t remaining();
  void jumpAhead(uint32_t units_ahead);

 private:
  uint32_t getSystemTime();

  bool is_disabled = 1;
  uint32_t start_time = 0;
  uint32_t value = 0;
  uint32_t default_value = 0;
};
