#ifndef CONF_H_
#define CONF_H_

#include <Arduino.h>
#include "structs.h"
#include "auto_config.h"

namespace conf {

  const ModeStruct* getMode(mode_enum mode);
  const KmapStruct* getKmap(mode_enum mode, seq_type_enum seq_type, uint8_t kmap_num);
  uint8_t getNumKmaps(mode_enum mode);

  const uint8_t* getAnagramMask(mode_enum mode);
  const uint8_t* getAnagram(mode_enum mode, uint8_t num);

  const uint8_t* getModChord(mode_enum mode, mod_enum modifier);
  const uint8_t getModifierkeyByte(uint8_t index);
  const mod_enum getModifierkeyEnum(uint8_t index);
  const mod_enum getWordmodEnum(uint8_t index);
  const uint8_t getModifierkeyIndex(mod_enum modifier);
  const mod_enum getNospaceEnum();
  const mod_enum getCapitalEnum();

}

#endif