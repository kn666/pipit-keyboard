#pragma once

#include <stdint.h>
#include "Chord.h"
#include "Comms.h"
#include "Feedback.h"
#include "History.h"
#include "auto_config.h"

class Sender {
 public:
  void deleteLastWord();
  void move(Motion motion, Direction direction);
  void backspace();

  void sendMods(const Chord* chord);
  void sendPlain(const Key* keys, uint8_t keys_length, const Chord* chord);
  void sendMacro(const Key* keys, uint8_t keys_length, const Chord* chord);
  void sendWord(const Key* keys, uint8_t keys_length, Chord* chord);
  void sendReport(Report* report);

  void releaseAll();
  void releaseNonMods();

  void setStickymod(uint8_t mod_byte);

  History history;

  Comms comms;

 private:
  void leftArrow();
  void rightArrow();
  void space();

  void sendKey(const Key* key);
  void sendKeyAndMod(uint8_t key_code, uint8_t mod_byte);
  void press(const Report* report);

  Report last_report;

  // TODO store stickymod as a Chord?
  uint8_t stickymod = 0;

  /***** for optimizing when to send releases *****/
  uint8_t last_sent_keycode = 0;
  uint8_t last_sent_mod_byte = 0;
};
