# Декодеры и границы Windows API — 0.3.0

Дата проверки: 2026-09-27. Декодеры написаны самостоятельно по публичным форматам
структур; исходники USBTreeView не используются. Raw-блоки не заменяются расшифровкой.

## Покрытие

| Группа | Смысловые поля |
|---|---|
| BOS | USB 2 Extension: LPM/BESL; SS: режимы/LTM/U1/U2; SSP: RX/TX sublink, speed ID, линии; Container ID; Platform UUID; PTM |
| CDC | Header, Call Management, ACM, Union, Country, Ethernet, NCM, MBIM и MBIM Extended |
| MIDI 1.0 | Streaming Header, IN/OUT Jack, связи pin/source, endpoint-associated Jack IDs |
| UAC 1/2 | AC Header, Input/Output Terminal, источники Mixer/Selector, Feature controls, UAC2 Clock Source/Selector/Multiplier; AS General, Type I format |
| UVC | VC Header/Input/Output Terminal/Camera/Processing Unit; VS Input/Output Header, uncompressed/MJPEG/frame-based format и frame, интервалы и FPS, Color Matching |
| HID | Windows HidP capabilities: button/value/link nodes; независимые IsRange/IsStringRange/IsDesignatorRange |

Режим из BOS — заявленная возможность, не измеренная скорость соединения.
Частоты/размеры кадра из UVC/UAC — объявленные режимы, не текущая настройка приложения.
UAC3, MIDI2, vendor-specific и неподдержанные подтипы остаются исходными байтами.
Усечённые известные массивы помечают конфигурацию неполной; bounded reads не выходят
за ответ. `Descriptor.complete` означает целостность ответа/структуры, а не полный
семантический разбор каждого возможного поля.

## HID Report Descriptor и MS OS

Windows принудительно заменяет `bmRequest` на `0x80`, `bRequest` на `0x06` в
[USB_DESCRIPTOR_REQUEST](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ns-usbioctl-_usb_descriptor_request).
Поэтому hub IOCTL не даёт корректный interface-recipient HID Report Descriptor и
произвольные MS OS vendor requests. Подмена драйвера/установка фильтра не выполняется.
В HID-полях это явно указано; HidP capabilities доступны независимо от сырого report.

Сборщик читает `HKLM\SYSTEM\CurrentControlSet\Control\usbflags\VIDPIDREV\osvc`.
Только точный двухбайтовый кэш с флагом поддержки 1 разрешает стандартное чтение
строки 0xEE. При отсутствии/отказе/неизвестном кэше повторный probe не выполняется.
Ответ обязан иметь 18 байт и сигнатуру MSFT100; иначе сохраняется как некорректный.
Vendor code отображается, но vendor-запрос не отправляется.

Источники:
- [Microsoft OS descriptors](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/microsoft-defined-usb-descriptors).
- [USB registry entries](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/usb-device-specific-registry-settings).
- [libwdi: WCID Devices](https://github.com/pbatard/libwdi/wiki/WCID-Devices) — порядок байтов osvc.
- [Linux USB ch9.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/usb/ch9.h),
  [cdc.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/usb/cdc.h),
  [audio.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/usb/audio.h),
  [audio-v2.h](https://github.com/torvalds/linux/blob/master/include/linux/usb/audio-v2.h),
  [midi.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/usb/midi.h),
  [video.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/usb/video.h).
  Для проверки смещений использованы установленные headers 7.0.0-34; исходники headers
  не копируются в продукт.

## Обновление при подключении

[CM_Register_Notification](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_register_notification)
на GUID_DEVINTERFACE_USB_DEVICE регистрируется до первого снимка. Callback только
устанавливает atomic flag. UI объединяет события за 800 мс, не запускает второй
сбор параллельно первому. Событие во время сбора вызывает следующий снимок.
Флажок «При подключении» отключает автоматический пересбор; ручной остаётся доступен.
Журнал показывает изменения состава только между завершёнными перечислениями.
Это не запись всех кратковременных переходов: события объединяются.

[CM_Unregister_Notification](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_unregister_notification)
вызывается вне callback и дожидается его окончания; память контекста освобождается
после снятия регистрации. Отказ регистрации отображается в статусной строке.
