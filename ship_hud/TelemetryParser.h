#pragma once
#include <Arduino.h>

class TelemetryParser {
public:
  typedef void (*FieldCallback)(uint8_t value);

  TelemetryParser() : _numFields(0), _bufPos(0) {}

  void onField(const char* key, FieldCallback cb) {
    if (_numFields >= MAX_FIELDS) return;
    strncpy(_fields[_numFields].key, key, KEY_LEN - 1);
    _fields[_numFields].key[KEY_LEN - 1] = '\0';
    _fields[_numFields].cb = cb;
    _numFields++;
  }

  void feed(Stream& stream) {
    while (stream.available()) {
      char c = stream.read();
      if (c == '\n' || c == '\r') {
        if (_bufPos > 0) {
          _buf[_bufPos] = '\0';
          parseLine(_buf);
          _bufPos = 0;
        }
      } else if (_bufPos < BUF_SIZE - 1) {
        _buf[_bufPos++] = c;
      }
    }
  }

private:
  static const int MAX_FIELDS = 16;
  static const int KEY_LEN = 4;
  static const int BUF_SIZE = 128;

  struct Field {
    char key[KEY_LEN];
    FieldCallback cb;
  };

  Field _fields[MAX_FIELDS];
  int _numFields;
  char _buf[BUF_SIZE];
  uint8_t _bufPos;

  void parseLine(char* line) {
    char* token = strtok(line, ",");
    while (token != NULL) {
      char* colon = strchr(token, ':');
      if (colon != NULL) {
        *colon = '\0';
        int val = atoi(colon + 1);
        if (val < 0) val = 0;
        if (val > 255) val = 255;
        dispatch(token, (uint8_t)val);
      }
      token = strtok(NULL, ",");
    }
  }

  void dispatch(const char* key, uint8_t value) {
    for (int i = 0; i < _numFields; i++) {
      if (strcmp(_fields[i].key, key) == 0) {
        _fields[i].cb(value);
        return;
      }
    }
  }
};
