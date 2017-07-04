#include "conf.h"

// using namespace conf;
namespace conf {

  const ModeStruct* getMode(mode_enum mode){
    return mode_structs[mode];
  }

  uint8_t getNumKmaps(mode_enum mode){
    return getMode(mode)->num_kmaps;
  }

  const KmapStruct* getKmap(mode_enum mode,
                      seq_type_enum seq_type,
                      uint8_t kmap_num){
    return getMode(mode)->kmaps[seq_type][kmap_num];
  }

  const uint8_t* getAnagramMask(mode_enum mode) {
    return getMode(mode)->anagram_mask;
  }

  const uint8_t* getAnagram(mode_enum mode, uint8_t num) {
    // TODO assert num < NUM_ANAGRAMS?
    return getMode(mode)->anagram_chords[num];
  }

  const uint8_t* getModChord(mode_enum mode, mod_enum modifier) {
    return getMode(mode)->mod_chords[modifier];
  }

  const uint8_t getModifierkeyByte(uint8_t index) {
    // TODO be consistent about mod terminology
    return modifierkey_keys[index];
  }

  const mod_enum getModifierkeyEnum(uint8_t index) {
    return (mod_enum) modifierkey_indices[index];
  }

  const mod_enum getWordmodEnum(uint8_t index) {
    return (mod_enum) wordmod_indices[index];
  }

  const uint8_t getModifierkeyIndex(mod_enum modifier) {
    for(uint8_t i = 0; i < NUM_MODIFIERKEYS; i++){
      if(modifierkey_indices[i] == modifier) {
        return i;
      }
    }
    DEBUG1_LN("ERROR: modifierkey index not found!");
    return 0; // might as well return something instead of panicking
  }

  const mod_enum getNospaceEnum() {
    return WORDMOD_NOSPACE_ENUM;
  }

  const mod_enum getCapitalEnum() {
    return WORDMOD_CAPITAL_ENUM;
  }

}
