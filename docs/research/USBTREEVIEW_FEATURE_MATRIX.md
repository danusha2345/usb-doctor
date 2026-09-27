# USBTreeView: покрытие функций и понятный аналог

Исследование от 2026-09-27. **Обновление реализации: базовый Windows-сборщик и нативный UI созданы; аппаратная проверка впереди.** Точный текущий объём: [IMPLEMENTATION.md](../IMPLEMENTATION.md). Интерактивный [прототип](../design/prototype.html) использует только демонстрационные данные; он не является функциональным аналогом USBTreeView. Дизайн и критерии описаны в [DESIGN.md](../design/DESIGN.md), API — в [WINDOWS_API_MAP.md](WINDOWS_API_MAP.md).

## Корпус и границы полноты

Проверены целиком [основная официальная страница](https://www.uwe-sieber.de/usbtreeview_e.html), [старая история](https://www.uwe-sieber.de/UsbTreeView_History_e.html) и статические ресурсы официального [x64 EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip) **4.7.4.0** (выпуск 22.08.2026). Извлечены 71 непустой узел главного меню, включая 59 конечных команд/вариантов, контекстные строки, семейства дескрипторов и шесть CLI-ключей. Windows 8+ — требование этой версии; автор также публикует Win32/ARM64 и старые сборки для предыдущих Windows. Это сведения о USBTreeView, не целевые платформы нашего MVP.

ZIP: 533531 байт, SHA256 `ec8e8e973f6f3e35e0188294c530e5ff584fd7a004f01d6c0f6adfbf46d543eb`. EXE: 1043472 байта, SHA256 `8027e88521238039544ac6c62bdb02a4d6e65de4a7d4ca60d0c49d935b1209c7`. URL, время получения и хеши страниц/меню сохранены в [SOURCES.json](SOURCES.json); raw-корпус находится в игнорируемой `.scratch/usbtreeview/`.

**F1 Help не извлечена и не прочитана.** ZIP не содержит отдельной TXT/CHM. Доступная web-история охватывает 1.1–2.6.1 и 4.4.0–4.7.4; промежуток 2.6.2–4.3.x не покрыт последовательной историей. Скрытые INI-настройки, все bitfields и runtime-доступность команд не установлены. Программа не запускалась на Windows. Статический пункт меню доказывает его присутствие, но не работоспособность. Поэтому ниже — полное покрытие обнаруженного корпуса, а не заявление о проверке абсолютно всех функций приложения.

## Этапы и единый статус

P0 — Windows API-spike; P1 — дизайн на демоданных; P2–P5 — сборщик, объяснения, отчёт, Windows alpha; P6 — техническая глубина и автоматизация; P7 — явные операции с устройствами; P8 — расширенная топология и платформы. Порядок и критерии в [PLAN.md](../../PLAN.md). Эта матрица фиксирует исследованный объём и целевой охват. T01/S01/W01/N01/X01 реализованы частично в первой сборке; построчный паритет с USBTreeView не подтверждён. Остальные функции остаются планом; актуальные границы перечислены в IMPLEMENTATION.md.

Обозначения источников: [O — официальная страница](https://www.uwe-sieber.de/usbtreeview_e.html), [H — старая история](https://www.uwe-sieber.de/UsbTreeView_History_e.html), [R — статические ресурсы EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip). В столбце UX указано проектное решение, а не функция конкурента.

## Матрица возможностей

| ID | Возможности USBTreeView / источник | Как сделать понятнее | Этап |
|---|---|---|---|
| T01 | Контроллеры, root/external hubs, порты, USB и PnP children [O] | Карточка изделия, раскрываемые функции; дерево для экспертов | P2; карточка — только демо P1 |
| T02 | Companion ports USB2/USB3, в том числе два у USB-C [O,R] | Физический разъём только при доказанной связи, иначе логические порты | P2/P6 |
| T03 | COM, диски/тома, WPD, drive numbers/sizes, hidden volumes [O,R] | «Доступен как COM3 / диск D:» с источником связи | P2, WPD и сложные корпуса P6 |
| T04 | USB4/Thunderbolt PnP, NVMe descendants [O] | Отдельно сведения Windows и неподтверждённые возможности туннелей | P8 |
| T05 | Bluetooth имена, MAC, последнее подключение, paired devices [O,R] | Отдельный раздел с приватностью MAC, не смешивать с физическим USB | P8 |
| S01 | Low/Full/High/Super/SSP, 10/20 Gbit/s на Windows 11 24H2+ [O] | Режим сейчас, возможности порта и устройства — разные поля | P0/P2; скорость — демо P1 |
| W01 | PnP ID/class/service/enumerator/driver/version/date [O,H] | «Windows» с объяснением problem code и техническим раскрытием | P2/P3 |
| W02 | HID и kernel streaming audio/video сведения [O] | Функции устройства и их параметры по запросу | P6 |
| E01 | Auto Refresh, сохранение выделения, arrival/removal, подсветка [O,R] | Спокойный журнал сеанса с временем; не перехватывать фокус | P2/P3; журнал — демо P1 |
| E02 | RR/RF safe removal, изменение problem state [O,R] | Текст состояния вместо буквенного кода; не диагноз кабеля | P3/P7 |
| N01 | Jump lists Drives/Others, поиск ID/drive/volume, история переходов [O] | Единый поиск устройств; расширенный поиск полей и назад/вперёд | P2/P6; поиск — демо P1 |
| N02 | Имена Windows/descriptor/smart; F2 rename [O,R] | Указать источник имени; локальный alias по умолчанию | P2; запись FriendlyName P7 |
| V01 | Empty ports/hubs, endpoints/children, auto-expand, limits 10/20/50/100 [R] | Пресеты «Обычно / Технически», пустое скрыто по умолчанию | P2/P6 |
| V02 | Шрифты/цвета панелей, High-DPI, Half-Dark, theme, topmost, single instance [O,R] | Системный масштаб/тема, доступность и компактность; дополнительные настройки отдельно | P1/P5/P6 |
| V03 | Highlight arrival/removal/problem/safe removal, duration/fade, companion colors [R] | Значение текстом + ненавязчивый цвет; отключаемая анимация | P2/P6 |
| X01 | Text/XML reports, XML import, selected subtree report [O,H,R] | JSON/HTML с предпросмотром и обезличиванием; совместимый XML через fixtures | P4, XML P6; экспорт демоданных P1 |
| X02 | Copy full/selected tree text/screenshot, full-height image [O,H,R] | «Скопировать сводку», расширенный формат в меню | P4/P6 |
| X03 | Шесть CLI-ключей поиска, отчёта, открытия, логирования [R] | Документированные команды, JSON/errors/exit codes и приватность | P6 |
| A01 | Device/Computer/Network Properties, Explorer, Regedit [O,R] | «Открыть в Windows» по конкретному объекту; Regedit в экспертном меню | P6 |
| A02 | Safe Remove и открытые handles при отказе [O] | Показать объект, затронутые функции и результат; полнота handles не обещана | P7 |
| A03 | Restart Device / Restart Port, restart as admin [O,R] | Отдельный helper по действию, список затронутых детей, результат возврата | P7 |
| A04 | Eject Media / Load Media [R] | Только для подходящего объекта, объяснить отличие от удаления устройства | P7 |
| A05 | Удаление Bluetooth pairing и descendants [O] | Отдельное явное действие с последствиями | P8 |
| A06 | Test Read Speed + progress/Stop [O,R] | Отдельный тест чтения; МБ/с всего тракта, лимит времени/объёма | P6 |
| A07 | Search online, Copy/Paste/Select All правой панели [O,R] | Локальный поиск первым; перед внешним поиском виден запрос без серийников | P6 |
| A08 | Системные tools shortcuts, Help/About [R] | Справка рядом с полем; «О программе» и служебные ссылки в меню | P5/P6 |
| Q01 | Неответивший descriptor / synthetic VID/PID, Intel phantom ports [O] | «Windows не получила данные», не выдумывать модель/физическую поломку | P0/P3 |
| Q02 | BadUSB эвристика неожиданных keyboard/network interfaces [O] | Показать состав функций; не обещать антивирус или вердикт вредоносности | P3/P6 |
| Q03 | Отключаемый 0xEE, suppression HID failures, vJoy blacklist [O,H] | Консервативный сбор; дополнительные запросы по выбору, журнал отказов | P0/P6 |

## Дескрипторная глубина

Следующие семейства найдены в ресурсах [EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip); UVC 1.5/H.264 также описаны на [странице автора](https://www.uwe-sieber.de/usbtreeview_e.html). Наличие имени типа не доказывает полное декодирование каждого подтипа. Для нашего приложения все декодеры пока **не реализованы**; техническая панель прототипа показывает только фикстуры.

| ID | Обнаруженное семейство | Подача и этап |
|---|---|---|
| D01 | Device, qualifier, configuration, other-speed, interface, IAD, endpoint, string | Структурированное раскрытие + raw; базовое P2, глубина P6 |
| D02 | Hub, extended/SuperSpeed hub, OTG, power | Связь с топологией, заявленные параметры; P6 |
| D03 | BOS: USB2 extension, SS/SSP capability, Container ID, Platform, Billboard/AUM, Firmware Status | Отделить capabilities от operating; P6 |
| D04 | BOS PD capability, battery info, consumer/provider port | Не текущий PD-контракт и не измерение питания; P6 |
| D05 | SS endpoint companion, SSP isochronous companion | Группировать с соответствующим endpoint; P6 |
| D06 | Audio 1.x/2: headers, terminals, mixer/selector/feature/processing/extension, formats/endpoints; Audio2 effect/clocks/rate converter/encoder | Раздел «Аудио» с расшифровкой единиц; P6 |
| D07 | MIDI header, IN/OUT jack, bulk endpoint | Раздел «MIDI», схемы связей; P6 |
| D08 | UVC control/streaming, terminals, selector/processing/extension, H.264 encoding, interrupts; uncompressed/frame-based/MJPEG/vendor/DV/MPEG2-TS/MPEG4-SL/H.264/simulcast/still/color | Таблица поддерживаемых форматов; источник дескриптор, не тест камеры; P6 |
| D09 | HID descriptor/report descriptor, расширенные HID сведения | Не читать report автоматически; учитывать известные сбои; P6 |
| D10 | CDC interface/header/union/ACM/call/line; названия ethernet/NCM/ATM/OBEX/MDLM/телефонных типов | Проверять поддержку каждого подтипа отдельно; P6 |
| D11 | UAS pipe usage, DFU functional, smart-card functional | DFU capability не означает прошивку устройства; P6 |
| D12 | Hex/ASCII dump, unknown descriptors | Сырые байты, длина, источник и честное unknown; P2/P6 |

## Все 59 конечных пунктов главного меню

Источник — `RT_MENU/105/0` из [EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip). Ниже полный путь, чтобы одинаковые названия не теряли смысл. Это перечень функций эталона; группа матрицы задаёт решение и этап. Базовый сбор/поиск/экспорт уже реализованы частично, но воспроизведение всех пунктов меню не заявляется.

| Resource ID | Путь меню | Группа |
|---|---|---|
| 40110 | File → Refresh · F5 | E01 / P2 |
| 40115 | File → Restart 'As Administrator' | A03 / P7 |
| 40127 | File → Open XML Report · Ctrl-O | X01 / P6 |
| 40125 | File → Save XML Report · Ctrl-S | X01 / P6 |
| 40120 | File → Save Text Report | X01 / P4 |
| 40100 | File → Exit · Alt-F4 | V02 / P5 |
| 40200 | Edit → Copy full Tree → As Text | X02 / P4–P6 |
| 40201 | Edit → Copy full Tree → As Screenshot | X02 / P4–P6 |
| 40202 | Edit → Copy Tree from selected Item → As Text | X02 / P4–P6 |
| 40203 | Edit → Copy Tree from selected Item → As Screenshot | X02 / P4–P6 |
| 40210 | Edit → Copy full Report · Ctrl+Shift+C | X02 / P4–P6 |
| 40211 | Edit → Copy Report from selected Item · Ctrl+Alt+Shift+C | X02 / P4–P6 |
| 40300 | Options → Auto Refresh | E01 / P2 |
| 40305 | Options → Endpoint Descriptors | D01,D12 / P6 |
| 40306 | Options → Read Msft String Descriptor 0xEE | Q03 / P6 |
| 40307 | Options → Scan all String Descriptors | Q03 / P6 |
| 40310 | Options → Descriptor HexDumps | D01,D12 / P6 |
| 40318 | Options → Child Devices in Tree | T01,T03,T05,V01 / P2–P8 |
| 40315 | Options → Hidden Volumes in Tree | T01,T03,T05,V01 / P2–P8 |
| 40316 | Options → Volumes' WPD in Tree | T01,T03,T05,V01 / P2–P8 |
| 40313 | Options → Drive Numbers in Tree | T01,T03,T05,V01 / P2–P8 |
| 40314 | Options → Drive Sizes in Tree | T01,T03,T05,V01 / P2–P8 |
| 40319 | Options → Bluetooth Device Names in Tree | T01,T03,T05,V01 / P2–P8 |
| 40317 | Options → Endpoints in Tree | T01,T03,T05,V01 / P2–P8 |
| 40340 | Options → Ports in Tree → Port Number | T02,V01 / P2–P6 |
| 40341 | Options → Ports in Tree → Port Chain | T02,V01 / P2–P6 |
| 40350 | Options → Device Names in Tree → From Device Manager | N02 / P2 |
| 40351 | Options → Device Names in Tree → From String Descriptors | N02 / P2 |
| 40352 | Options → Device Names in Tree → Smart Choice | N02 / P2 |
| 40320 | Options → Expand for empty Ports | V01,E01 / P2–P6 |
| 40321 | Options → Expand for empty Hubs | V01,E01 / P2–P6 |
| 40330 | Options → Expand for Child Devices → Never | V01,E01 / P2–P6 |
| 40331 | Options → Expand for Child Devices → For up to 10 Devs | V01,E01 / P2–P6 |
| 40332 | Options → Expand for Child Devices → For up to 20 Devs | V01,E01 / P2–P6 |
| 40333 | Options → Expand for Child Devices → For up to 50 Devs | V01,E01 / P2–P6 |
| 40334 | Options → Expand for Child Devices → For up to 100 Devs | V01,E01 / P2–P6 |
| 40335 | Options → Expand for Child Devices → Always | V01,E01 / P2–P6 |
| 40322 | Options → Expand for new USB Devices | V01,E01 / P2–P6 |
| 40323 | Options → Expand for new Child Devices | V01,E01 / P2–P6 |
| 40324 | Options → Jump to new USB Devices | V01,E01 / P2–P6 |
| 40325 | Options → Jump to removed USB Devices | V01,E01 / P2–P6 |
| 40361 | Options → Fonts... → Left Pane... | V02 / P5–P6 |
| 40360 | Options → Fonts... → Right Pane... | V02 / P5–P6 |
| 40363 | Options → Background Colors... → Left Pane... | V02 / P5–P6 |
| 40362 | Options → Background Colors... → Right Pane... | V02 / P5–P6 |
| 40375 | Options → Highlighting... | V03 / P6 |
| 40382 | Options → Half-Dark Mode | V02 / P5–P6 |
| 40381 | Options → Windows Theme | V02 / P5–P6 |
| 40385 | Options → Allow only one Instance | V02 / P5–P6 |
| 40390 | Options → Always On Top | V02 / P5–P6 |
| 40400 | Tools → Bluetooth Devices | T05,A05 / P8 |
| 40401 | Tools → Device Manager | A08 / P5–P6 |
| 40402 | Tools → Disk Management | A08 / P5–P6 |
| 40403 | Tools → Event Viewer | A08 / P5–P6 |
| 40404 | Tools → Network Adapter | A08 / P5–P6 |
| 40405 | Tools → Bluetooth Settings | A08 / P5–P6 |
| 40406 | Tools → Computer Mangement | A08 / P5–P6 |
| 40900 | Help → Help | A08 / P5–P6 |
| 40901 | Help → About | A08 / P5–P6 |

## Дополнительные действия, настройки и CLI

### Настройки подсветки
Диалог resource `RT_DIALOG/3000/1033`: длительность основной подсветки и fading; отдельные цвета Arrived, Removed, Got Problem, Safely Removed; highlight selected item, ensure visible; цвета High-Speed/SuperSpeed companion, режим no/permanent/fading highlight. Наличие подтверждено статически. — [EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip).

### Контекстные действия и связанные диалоги
- Device Properties, Properties узла компьютера, Network Properties; открыть Explorer для томов/WPD; Regedit для device-specific keys. — [Страница](https://www.uwe-sieber.de/usbtreeview_e.html).
- Safe Remove, Restart Device, Restart Port; при неудачном удалении отображаются открытые handles. Это действия со состоянием устройства, а не диагностика без изменений. — [Страница](https://www.uwe-sieber.de/usbtreeview_e.html).
- Eject Media/Load Media найдены как контекстные строки; Copy Report from here; Copy Tree from here text/screenshot. — [EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip).
- Test Read Speed для диска: короткий тест чтения, диалог с прогрессом и Stop; точный размер буфера/продолжительность/правила доступа статически не установлены. — [Страница](https://www.uwe-sieber.de/usbtreeview_e.html), [EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip).
- Bluetooth Devices: список и удаление сопряжённых устройств, при этом Windows удаляет descendent devices. — [Страница](https://www.uwe-sieber.de/usbtreeview_e.html).
- F2 или удержание повторного одиночного клика ~1 секунду переименовывает. В режиме From Device Manager также записывается FriendlyName в registry; в иных режимах имя остаётся только в USBTreeView. — [Страница](https://www.uwe-sieber.de/usbtreeview_e.html).
- Copy/Paste/Select All/Search online найдены в строках правой панели; online search также заявлен в 4.7.2. Точный URL-провайдер и состав передаваемой строки не установлен. — [Страница](https://www.uwe-sieber.de/usbtreeview_e.html), [EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip).

### Отчёты, копирование и CLI
- Текстовый отчёт всего дерева/ветки/устройства, XML запись/чтение (формат не совместим с USBView); копирование дерева полностью/от выбранного узла текстом или изображением; полный по высоте screenshot дерева. В 4.7.4 исправлено чтение XML, сломанное в 4.7.2. — [Страница](https://www.uwe-sieber.de/usbtreeview_e.html), [история](https://www.uwe-sieber.de/UsbTreeView_History_e.html).
- CLI usage официального EXE перечисляет 6 ключей: `-S=<SearchFor>` — поиск, `-R=<ReportFile>` — текстовый отчёт, `-X=<ReportFile>` — XML отчёт, `-O=<ReportFile>` — открыть XML, `-L=<LogFile>` — лог, `-F` — flush после каждой строки. Это extract строки Usage, команды не запускались. Старый `/R:path` также документирован в истории, его точная совместимость сегодня не проверена. — [EXE](https://www.uwe-sieber.de/files/UsbTreeView_x64.zip), [история](https://www.uwe-sieber.de/UsbTreeView_History_e.html).


Все перечисленные дополнительные действия включены в группы V03, A01–A07 и X01–X03. Для CLI первой собственной версии планируются эквивалентные сценарии, а не неподтверждённая побайтовая совместимость со всеми флагами USBTreeView. Неизвестные INI-опции не придумывать.

## Условия использования и граница заимствования

USBTreeView — freeware; автор разрешает коммерческое использование и распространение, включая пакеты, запрещает изменение файлов и распространение через downloader. Это **не open-source лицензия** на код и не разрешение модифицировать EXE. [Условия автора](https://www.uwe-sieber.de/usbtreeview_e.html). Microsoft USBView из Windows-driver-samples опубликован под **MS-PL**, не MIT; заимствование проверять по конкретным файлам и сохранять требуемые notices. [Microsoft LICENSE](https://raw.githubusercontent.com/microsoft/Windows-driver-samples/main/LICENSE).

## Что должно быть лучше и как это проверить

Преимущество проектируется в точной интерпретации и сравнении, а не в скрытии данных. Три слоя: понятная карточка → основание вывода и структурированные подробности → raw и экспертные операции. Порт/кабель сравниваются с изменением одной переменной; неоднозначность идентичности требует ручного подтверждения. Журнал ограничен периодом наблюдения. Неизвестное, отказ доступа и неподдерживаемое поле не превращаются в диагноз неисправности. Режим соединения, заявленные возможности и измеренный read throughput всегда раздельны.

Целевые критерии, ещё не измеренные: 5–10 пользователей выполняют одинаковые задачи в USBTreeView и USB-докторе; за 30 секунд находят устройство и режим, за 2 минуты завершают сравнение, не выдают unknown за поломку. Измерять правильность объяснения, время и ложные диагнозы. «Лёгкий» продукт подтвердить cold start, idle CPU/RAM, размером сборки и поведением при 100+ функциях; сам выбор Rust этого не доказывает.
