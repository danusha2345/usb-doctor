// Public DEVPROPKEY identifiers from windows-sys 0.61.2; human labels for display.
pub fn label(guid: u128, pid: u32) -> (&'static str, &'static str) {
    match (guid, pid) {
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 19) => (
            "AdditionalSoftwareRequested",
            "Windows · AdditionalSoftwareRequested",
        ),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 30) => ("Address", "Адрес на шине"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 24) => {
            ("AssignedToGuest", "Windows · AssignedToGuest")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 38) => {
            ("BaseContainerId", "Windows · BaseContainerId")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 10) => {
            ("BiosDeviceName", "Windows · BiosDeviceName")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 23) => ("BusNumber", "Номер шины"),
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 7) => ("BusRelations", "Windows · BusRelations"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 4) => {
            ("BusReportedDeviceDesc", "Описание от шины")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 21) => ("BusTypeGuid", "Windows · BusTypeGuid"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 17) => ("Capabilities", "Capabilities"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 29) => {
            ("Characteristics", "Windows · Characteristics")
        }
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 9) => ("Children", "Дочерние PnP"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 9) => ("Class", "Класс Windows"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 10) => ("ClassGuid", "GUID класса"),
        (0x6a742654_d0b2_4420_a523_e068352ac1df, 2) => ("CompanionApps", "Windows · CompanionApps"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 4) => ("CompatibleIds", "Compatible IDs"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 12) => ("ConfigFlags", "Windows · ConfigFlags"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 7) => {
            ("ConfigurationId", "Windows · ConfigurationId")
        }
        (0x8c7ed206_3f8a_4827_b3ab_ae9e1faefc6c, 2) => ("ContainerId", "Container ID"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 25) => {
            ("CreatorProcessId", "Windows · CreatorProcessId")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 2) => {
            ("DHP_Rebalance_Policy", "Windows · DHP_Rebalance_Policy")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 12) => ("DebuggerSafe", "Windows · DebuggerSafe"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 21) => {
            ("DependencyDependents", "Windows · DependencyDependents")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 20) => {
            ("DependencyProviders", "Windows · DependencyProviders")
        }
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 2) => ("DevNodeStatus", "Статус DevNode"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 27) => ("DevType", "Windows · DevType"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 2) => ("DeviceDesc", "Описание Windows"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 11) => ("Driver", "Windows · Driver"),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 11) => {
            ("DriverCoInstallers", "Windows · DriverCoInstallers")
        }
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 2) => ("DriverDate", "Дата драйвера"),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 4) => ("DriverDesc", "Описание драйвера"),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 5) => ("DriverInfPath", "INF драйвера"),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 6) => ("DriverInfSection", "Секция INF"),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 7) => {
            ("DriverInfSectionExt", "Windows · DriverInfSectionExt")
        }
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 15) => {
            ("DriverLogoLevel", "Windows · DriverLogoLevel")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 11) => {
            ("DriverProblemDesc", "Windows · DriverProblemDesc")
        }
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 10) => {
            ("DriverPropPageProvider", "Windows · DriverPropPageProvider")
        }
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 9) => ("DriverProvider", "Поставщик драйвера"),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 14) => ("DriverRank", "Windows · DriverRank"),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 3) => ("DriverVersion", "Версия драйвера"),
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 4) => {
            ("EjectionRelations", "Windows · EjectionRelations")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 24) => ("EnumeratorName", "Enumerator"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 28) => ("Exclusive", "Windows · Exclusive"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 23) => {
            ("ExtendedAddress", "Windows · ExtendedAddress")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 15) => (
            "ExtendedConfigurationIds",
            "Windows · ExtendedConfigurationIds",
        ),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 17) => ("FirmwareDate", "Windows · FirmwareDate"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 19) => {
            ("FirmwareRevision", "Windows · FirmwareRevision")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 26) => {
            ("FirmwareVendor", "Windows · FirmwareVendor")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 18) => {
            ("FirmwareVersion", "Windows · FirmwareVersion")
        }
        (0x83da6326_97a6_4088_9453_a1923f573b29, 101) => {
            ("FirstInstallDate", "Windows · FirstInstallDate")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 14) => ("FriendlyName", "Название Windows"),
        (0x80d81ea6_7473_4b0c_8216_efc11a2c4c8b, 3) => {
            ("FriendlyNameAttributes", "Windows · FriendlyNameAttributes")
        }
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 18) => {
            ("GenericDriverInstalled", "Windows · GenericDriverInstalled")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 3) => ("HardwareIds", "Hardware IDs"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 6) => ("HasProblem", "Windows · HasProblem"),
        (0x8c7ed206_3f8a_4827_b3ab_ae9e1faefc6c, 4) => (
            "InLocalMachineContainer",
            "Windows · InLocalMachineContainer",
        ),
        (0x83da6326_97a6_4088_9453_a1923f573b29, 100) => ("InstallDate", "Windows · InstallDate"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 36) => ("InstallState", "Windows · InstallState"),
        (0x78c34fc8_104a_4aca_9ea4_524d52996e57, 256) => ("InstanceId", "Instance ID"),
        (0x80d81ea6_7473_4b0c_8216_efc11a2c4c8b, 7) => (
            "IsAssociateableByUserAction",
            "Windows · IsAssociateableByUserAction",
        ),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 5) => ("IsPresent", "Windows · IsPresent"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 16) => {
            ("IsRebootRequired", "Windows · IsRebootRequired")
        }
        (0x83da6326_97a6_4088_9453_a1923f573b29, 102) => {
            ("LastArrivalDate", "Windows · LastArrivalDate")
        }
        (0x83da6326_97a6_4088_9453_a1923f573b29, 103) => {
            ("LastRemovalDate", "Windows · LastRemovalDate")
        }
        (0x80497100_8c73_48b9_aad9_ce387e19c56e, 3) => ("Legacy", "Windows · Legacy"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 22) => {
            ("LegacyBusType", "Windows · LegacyBusType")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 15) => ("LocationInfo", "Размещение"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 37) => ("LocationPaths", "Пути размещения"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 20) => ("LowerFilters", "Windows · LowerFilters"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 13) => ("Manufacturer", "Производитель Windows"),
        (0x80d81ea6_7473_4b0c_8216_efc11a2c4c8b, 4) => {
            ("ManufacturerAttributes", "Windows · ManufacturerAttributes")
        }
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 8) => {
            ("MatchingDeviceId", "Windows · MatchingDeviceId")
        }
        (0x78c34fc8_104a_4aca_9ea4_524d52996e57, 39) => ("Model", "Windows · Model"),
        (0x80d81ea6_7473_4b0c_8216_efc11a2c4c8b, 2) => ("ModelId", "Windows · ModelId"),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 17) => {
            ("NoConnectSound", "Windows · NoConnectSound")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 3) => ("Numa_Node", "Windows · Numa_Node"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 1) => {
            ("Numa_Proximity_Domain", "Windows · Numa_Proximity_Domain")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 16) => ("PDOName", "Windows · PDOName"),
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 8) => ("Parent", "Родитель PnP"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 9) => {
            ("PhysicalDeviceLocation", "Windows · PhysicalDeviceLocation")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 13) => {
            ("PostInstallInProgress", "Windows · PostInstallInProgress")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 32) => ("PowerData", "Данные питания Windows"),
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 6) => {
            ("PowerRelations", "Windows · PowerRelations")
        }
        (0x80d81ea6_7473_4b0c_8216_efc11a2c4c8b, 5) => {
            ("PresenceNotForDevice", "Windows · PresenceNotForDevice")
        }
        (0x6a742654_d0b2_4420_a523_e068352ac1df, 3) => {
            ("PrimaryCompanionApp", "Windows · PrimaryCompanionApp")
        }
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 3) => ("ProblemCode", "Код проблемы"),
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 12) => {
            ("ProblemStatus", "Windows · ProblemStatus")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 33) => ("RemovalPolicy", "Политика удаления"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 34) => {
            ("RemovalPolicyDefault", "Windows · RemovalPolicyDefault")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 35) => {
            ("RemovalPolicyOverride", "Windows · RemovalPolicyOverride")
        }
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 5) => {
            ("RemovalRelations", "Windows · RemovalRelations")
        }
        (0x80497100_8c73_48b9_aad9_ce387e19c56e, 2) => ("Reported", "Windows · Reported"),
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 8) => {
            ("ReportedDeviceIdsHash", "Windows · ReportedDeviceIdsHash")
        }
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 13) => (
            "ResourcePickerExceptions",
            "Windows · ResourcePickerExceptions",
        ),
        (0xa8b865dd_2e3d_4094_ad97_e593a70c75d6, 12) => {
            ("ResourcePickerTags", "Windows · ResourcePickerTags")
        }
        (0xafd97640_86a3_4210_b67c_289c41aabe55, 2) => {
            ("SafeRemovalRequired", "Windows · SafeRemovalRequired")
        }
        (0xafd97640_86a3_4210_b67c_289c41aabe55, 3) => (
            "SafeRemovalRequiredOverride",
            "Windows · SafeRemovalRequiredOverride",
        ),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 25) => ("Security", "Windows · Security"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 26) => ("SecuritySDS", "Windows · SecuritySDS"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 6) => ("Service", "Служба"),
        (0x83da6326_97a6_4088_9453_a1923f573b29, 6) => ("SessionId", "Windows · SessionId"),
        (0x80d81ea6_7473_4b0c_8216_efc11a2c4c8b, 8) => {
            ("ShowInUninstallUI", "Windows · ShowInUninstallUI")
        }
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 10) => ("Siblings", "Windows · Siblings"),
        (0x80d81ea6_7473_4b0c_8216_efc11a2c4c8b, 6) => {
            ("SignalStrength", "Windows · SignalStrength")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 22) => {
            ("SoftRestartSupported", "Windows · SoftRestartSupported")
        }
        (0x540b947e_8b40_45bc_a8a2_6a0b894cbda2, 14) => ("Stack", "Windows · Stack"),
        (0x4340a6c5_93fa_4706_972c_7b648008a5a7, 11) => {
            ("TransportRelations", "Windows · TransportRelations")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 18) => ("UINumber", "Windows · UINumber"),
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 31) => {
            ("UINumberDescFormat", "Windows · UINumberDescFormat")
        }
        (0xa45c254e_df1c_4efd_8020_67d146a850e0, 19) => ("UpperFilters", "Windows · UpperFilters"),
        _ => ("Unknown", "Свойство Windows"),
    }
}
