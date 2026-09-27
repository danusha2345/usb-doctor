# Windows API и границы понятной USB-диагностики

## Что технически реализуемо

Понятный USB-доктор реалистичен как приложение user-mode, использующее USB hub IOCTL и PnP-модель Windows. Ниже подтверждённые API отделены от предложенного UX; базовый сборщик уже реализован (см. [IMPLEMENTATION.md](../IMPLEMENTATION.md)), аппаратного паритета с USBTreeView пока нет.

### Подтверждено документацией

| Область | Подтверждённый технический путь | Предлагаемая подача / граница |
|---|---|---|
| 1. Контроллеры, хабы, порты | Microsoft USBView перечисляет host controllers, root/external hubs, затем downstream ports; открывает handles и вызывает hub IOCTL. [USBView sample](https://learn.microsoft.com/en-us/samples/microsoft/windows-driver-samples/usbview-sample-application/) | Карточки устройств; цепочка подключения вместо обязательного дерева всех пустых портов |
| 2. Название устройства | Windows предоставляет FriendlyName и BusReportedDeviceDesc. [FriendlyName](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devpkey-device-friendlyname), [bus description](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devpkey-device-busreporteddevicedesc) | Показать происхождение имени; собственный псевдоним хранить локально, не менять реестр автоматически |
| 3. Составное устройство | ContainerID группирует функции физического изделия. [USB ContainerIDs](https://learn.microsoft.com/en-us/windows-hardware/drivers/usbcon/usb-containerids-in-windows) | Одна карточка изделия, внутри функции: аудио, HID, накопитель, сеть |
| 4. Идентичность | Одинаковые серийники/ContainerID вызывают конфликт и объединение функций разных физических устройств. [Microsoft](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/avoiding-device-container-conflicts) | VID/PID не использовать как уникальный ключ; уверенность сопоставления и ручное подтверждение при неоднозначности |
| 5. Текущая скорость | EX.Speed ограничен HighSpeed; Microsoft показывает коррекцию по EX_V2 OperatingAtSuperSpeed/Plus. [EX](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ns-usbioctl-_usb_node_connection_information_ex) | Без EX_V2 не объявлять USB3 устройство медленным только из-за EX.Speed=HighSpeed |
| 6. Возможности скорости | EX_V2 отдельно содержит capable и operating флаги; SupportedUsbProtocols относится к порту. [EX_V2](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ni-usbioctl-ioctl_usb_get_node_connection_information_ex_v2), [flags](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ns-usbioctl-_usb_node_connection_information_ex_v2_flags) | Отдельные поля «сейчас», «возможности устройства», «возможности порта»; unknown не превращать в false |
| 7. SuperSpeedPlus | Отдельный user-mode IOCTL возвращает super-speed lane information. [SSP IOCTL](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ni-usbioctl-ioctl_usb_get_node_connection_superspeedplus_information) | Показывать точные числа только при подтверждённых lane данных; иначе «SuperSpeedPlus, точный режим недоступен» |
| 8. Физические разъёмы | USB_PORT_CONNECTOR_PROPERTIES содержит companion hub/port; USB3 hub имеет две независимо перечисляемые части. [Connector properties](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ns-usbioctl-_usb_port_connector_properties) | Не рисовать два логических порта как два физических; при отсутствии данных оставлять неизвестную связь |
| 9. USB4/Thunderbolt | Windows 11 использует системный connection manager, отдельные host/device routers и драйверы. [USB4](https://learn.microsoft.com/en-us/windows-hardware/design/component-guidelines/usb4-intro-to-connection-manager) | Отдельная ветвь модели, не пытаться получить весь USB4 через USB3 hub IOCTL |
| 10. Дескрипторы | USBView читает configuration/string descriptors через IOCTL_USB_GET_DESCRIPTOR_FROM_NODE_CONNECTION. [Sample](https://learn.microsoft.com/en-us/samples/microsoft/windows-driver-samples/usbview-sample-application/) | Базовые данные автоматически; дополнительные подробности по запросу; bounds checks, частичный результат |
| 11. Статус Windows | CM_Get_DevNode_Status возвращает DN_* и CM_PROB_*; problem code действителен только при DN_HAS_PROBLEM. [API](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_get_devnode_status) | Код + понятное значение + отдельные гипотезы; Code 43 не диагноз кабеля |
| 12. События | CM_Register_Notification работает с Windows 8; сначала регистрируют callback, потом получают существующие интерфейсы; возможны дубли arrival. [API](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_register_notification) | Очередь событий и dedup; callback без блокирующих I/O; журнал только с момента наблюдения |
| 13. Диски и разделы | IOCTL_STORAGE_GET_DEVICE_NUMBER возвращает device/partition numbers. [API](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddstor/ni-ntddstor-ioctl_storage_get_device_number) | Связать PnP descendants с дисками и томами; не считать букву диска постоянным ID |
| 14. COM и иные функции | USBTreeView сопоставляет child devices, COM и drive letters. [USBTreeView](https://www.uwe-sieber.de/usbtreeview_e.html) | Нужен API-spike PnP descendants + COM interface properties; не открывать COM-порт для диагностики его наличия |
| 15. Питание | Configuration descriptor содержит declared MaxPower и bmAttributes. [Descriptor](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbspec/ns-usbspec-_usb_configuration_descriptor) | «Заявлено в дескрипторе», без датчика реального тока/напряжения и без обещания PD-контракта |
| 16. Тест чтения | CreateFile поддерживает чтение устройства/файла; NO_BUFFERING имеет требования к alignment и не отключает hardware cache. [CreateFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew), [Buffering](https://learn.microsoft.com/en-us/windows/win32/fileio/file-buffering) | Отдельное явное действие, bounded bytes/time, только чтение; результат относится ко всему тракту, не только USB |
| 17. Безопасное извлечение | CM_Request_Device_Eject возвращает veto type/name; в отдельных сессиях требуются привилегии. [API](https://learn.microsoft.com/en-us/windows/win32/api/cfgmgr32/nf-cfgmgr32-cm_request_device_ejectw) | Показать отказ и доступное объяснение. Veto name не считать гарантированным полным списком удерживающих процессов |
| 18. Перезапуск | IOCTL_USB_HUB_CYCLE_PORT адресует hub и требует администратора на Windows 8+. DIF_PROPERTYCHANGE — отдельная операция PnP, может потребовать reboot. [Cycle port](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ni-usbioctl-ioctl_usb_hub_cycle_port), [DIF](https://github.com/MicrosoftDocs/windows-driver-docs/blob/staging/windows-driver-docs-pr/install/dif-propertychange.md) | Поздний этап: точная цель, дочерние устройства, предупреждение о разрыве связи, проверка возвращения; не обещать физическое снятие VBUS на любом железе |
| 19. Снимки и импорт | USBTreeView экспортирует и читает XML. [USBTreeView](https://www.uwe-sieber.de/usbtreeview_e.html) | Наш schema-versioned JSON/HTML и сравнение; импорт XML — отдельный parser, не обещать совместимость без реальных fixtures |
| 20. Доступность UI | egui предоставляет AccessKit; custom widgets требуют семантики. Проект прямо предупреждает, что egui не предназначен для native look. [Accessibility](https://github.com/emilk/egui/blob/main/docs/accessibility.md), [egui](https://github.com/emilk/egui) | Rust + egui оставить гипотезой до проверки Narrator/NVDA, focus, keyboard, DPI и реального потребления ресурсов |

### Проектные решения

- API-слой стоит разделить на PnP inventory, hub topology, enriched descriptors, events, optional operations. Ошибка в одном сборщике не должна стирать доступную карточку.
- Каждое поле: value, source, timestamp, availability (present/unsupported/access_denied/device_gone/malformed/not_requested), а не просто Option. Это проектное предложение.
- Не создавать общий «health score 98%»: набор доступных проверок различается, сведения Windows не доказывают физическую исправность.
- Ограничения таймаута UI не гарантируют отмену уже зависшего драйверного запроса. Рассмотреть worker process после API-spike; не обещать абсолютную защиту от неисправного kernel driver.
- Разрыв связи и сон/пробуждение должны обрабатываться как изменение данных, не как авария приложения. Аномальные события не автоматически считать физическими выдёргиваниями кабеля.

### Не проверено

- Не было запуска на Windows, реальных USB устройств и измерений прав доступа. Перечисленные API подтверждены документацией, их сочетание в нашем приложении ещё не проверено.
- Полный контракт сопоставления COM interfaces, WPD storage, нескольких дисков одного корпуса, USB4 tunneled NVMe нуждается в spike и отдельном наборе fixtures.
- Надёжный общий user-mode способ получить реальный PD voltage/current и проверить качество USB-сигнала не установлен.
- Для MaxPower нельзя механически распространять единицы из одной страницы Microsoft на все версии USB; сверить USB2/USB3 spec и тестовые дампы до отображения mA.

## Какие ошибки особенно важно предотвратить

Простой текст должен быть точнее технического дампа, а не категоричнее его. Главное преимущество проектируется в корректной интерпретации, контролируемом сравнении и понятном следующем действии.

### Подтверждено документацией

- bcdUSB означает версию спецификации дескриптора, а не измеренную скорость. iSerialNumber — индекс строки, а не сам серийник. [USB_DEVICE_DESCRIPTOR](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbspec/ns-usbspec-_usb_device_descriptor)
- EX.Speed без поправки EX_V2 способен ввести в заблуждение; SSP flags тоже не заменяют lane query для точного режима. [EX](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ns-usbioctl-_usb_node_connection_information_ex), [SSP](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/usbioctl/ni-usbioctl-ioctl_usb_get_node_connection_superspeedplus_information)
- ContainerID может создаваться ОС из серийника либо быть случайным, если подходящего идентификатора нет. [Microsoft](https://learn.microsoft.com/sr-latn-rs/windows-hardware/drivers/install/how-usb-devices-are-assigned-container-ids)
- USBTreeView 4.7.4 уже предлагает high-DPI, поиск, jump lists, переименование, USB4 PnP, классические/расширенные дескрипторы, XML, тест чтения, извлечение и рестарты. История содержит сбои HID-запросов вплоть до перезапуска устройств и проблему vJoy с BSOD; приложение подавляет проблемные запросы. [Официальная страница](https://www.uwe-sieber.de/usbtreeview_e.html)

### Проектные решения

- Иерархия скорости: точные SSP lane сведения → EX_V2 operating flags → применимый EX.Speed. Хранить исходные поля и противоречия; конфликт не скрывать красивой цифрой.
- Сравнение двух подключений должно отдельно показывать уверенность идентичности: подтверждено пользователем / сильное совпадение / несколько кандидатов. VID/PID и одинаковое имя не достаточны.
- Из «480 Мбит/с сейчас» + «подтверждена SuperSpeed capability» следует предложение сравнить кабель/порт; не следует «кабель плохой». Одновременная смена кабеля и порта не выделяет причину.
- Счётчик reconnect назвать «повторных обнаружений за сеанс», если не обеспечена дедупликация composite-child событий до уровня физического изделия.
- Не показывать сырой descriptor query как гарантированно безвредное «чтение Windows»: часть сведений требует обращения к реальному устройству.
- Для экспертного режима сохранить сырые поля и ошибки; простая карточка должна позволять раскрыть основание вывода, а не отрезать техническую глубину.

### Не проверено

- Статические ресурсы USBTreeView и все найденные пункты меню сведены в [матрицу](USBTREEVIEW_FEATURE_MATRIX.md). F1 Help и runtime-поведение не проверены.
- Нет доказательства превосходства нашего UX: для этого нужны прототип, задания и сравнительное пользовательское тестирование.

## Как доставлять по этапам и проверять

Рекомендуется охватить всю матрицу дорожной картой, но сначала доказать точность read-only снимка и понятность трёх сценариев. Дескрипторная глубина, операции и USB4 должны иметь отдельные критерии готовности.

### Подтверждено документацией

- Windows-driver-samples опубликован под MS-PL; файл LICENSE требует сохранения notices и оговаривает распространение исходников/бинарников. Не считать его MIT и проверять лицензию конкретных заимствуемых файлов. [LICENSE](https://raw.githubusercontent.com/microsoft/Windows-driver-samples/main/LICENSE)
- USBTreeView — freeware с разрешённым использованием/распространением, включая коммерческое, но запретом модификации его файлов. Это не лицензия на произвольное заимствование исходников. [Условия автора](https://www.uwe-sieber.de/usbtreeview_e.html)
- egui имеет MIT OR Apache-2.0, включая отдельные лицензии комплектных шрифтов; AccessKit не освобождает приложение от задания accessible names. [egui](https://github.com/emilk/egui), [Accessibility](https://github.com/emilk/egui/blob/main/docs/accessibility.md)

### Проектные решения

Предложенные этапы:

1. **API-spike.** JSON без сложного UI: PnP nodes, hubs, EX/V2/SSP, error states, стандартные права. Сверка одного и того же подключения с USBTreeView и Windows Device Manager. Сохранять наблюдения, не реальные серийники в Git.
2. **MVP.** Карточка, цепочка, скорость с основаниями, Windows status, COM/disk по доказанным связям, журнал сеанса, два снимка, мастер сравнения, обезличенный отчёт и демо. Экспертная панель сырых полей.
3. **Диагностическая глубина.** Декодеры классов и HID/UVC/audio по безопасной политике запросов; bounded retries и suppression после отказа. Настоящий XML importer, расширенное сопоставление WPD/USB4 и отдельный read benchmark.
4. **Операции.** Safe removal с veto, затем restart device/port через отдельно запускаемый privileged helper. До операции отображать затрагиваемые дочерние устройства. Handles attribution — исследовательская подзадача, не обещание первой версии.
5. **Полнота.** USB4 routers/tunnels, Bluetooth descendants, ARM64, системные tools shortcuts, CLI/export automation только после документированного спроса и покрытия.

Предложенные fixtures и аппаратные сценарии:

- HighSpeed device на USB-C без SuperSpeed: результат «нормальный доступный режим», не красная тревога.
- SSD через SuperSpeed кабель и USB2 кабель; затем тот же кабель через два разных порта: раздельное изменение переменных.
- EX.HighSpeed + V2.SuperSpeedOperating; V2 access denied; SSP unsupported; противоречащие поля; unknown enum values.
- Один USB3 hub как две логические половины; несколько companion ports; отсутствующая/противоречащая firmware mapping.
- Composite audio + HID, USB serial adapter, Bluetooth COM descendant, storage с несколькими томами, WPD phone, USB4 NVMe.
- Два одинаковых изделия без серийников; повторённый серийник; смена порта; устройство меняет PID после firmware mode switch.
- Truncated descriptor, bLength=0, huge wTotalLength, invalid UTF-16, исчезновение устройства между двумя вызовами, event flood, sleep/resume.
- Простые права, удалённая сессия, запрет handle open; raw errors остаются доступны и UI не зависает.
- Safe removal blocked, unplug во время чтения, отказ privileged helper, перезапуск хаба с несколькими детьми — только на специально выбранном стенде.
- DPI 100/150/200%, переход между мониторами, длинное русское имя, клавиатура без мыши, Narrator/NVDA, high contrast, пустой список, 100+ функций.

Предложенные измеримые UX-критерии: пользователь за 30 секунд находит устройство и текущий режим; отличает режим соединения от скорости чтения; за 2 минуты проводит сравнение; в контрольных unknown fixtures ни одного ложного «неисправен». Проверять 5–10 пользователями на одинаковых задачах против USBTreeView, фиксируя ошибки и время. Это целевые значения, не результаты.

Для «лёгкого» UI также нужны собственные замеры cold start, idle CPU, memory, binary size и нагрузка при потоке PnP событий. Не заявлять лёгкость только из-за выбора Rust.

### Не проверено

- Этапы, тесты и UX-критерии являются предложением; оценка сроков невозможна до Windows API-spike и проверки UI framework.
- Исследование не заменяет лицензионную проверку конкретного релизного состава зависимостей.
