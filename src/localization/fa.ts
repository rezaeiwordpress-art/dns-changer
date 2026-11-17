import type { ErrorCode } from '../types/dns';
import type { Ipv4ValidationErrorKey } from '../utils/validation';

export interface Translations {
  appName: string;
  appTagline: string;
  nav: {
    dashboard: string;
    providers: string;
    customDns: string;
    adapters: string;
    settings: string;
    about: string;
  };
  common: {
    refresh: string;
    refreshing: string;
    cancel: string;
    confirm: string;
    save: string;
    add: string;
    edit: string;
    delete: string;
    close: string;
    select: string;
    selected: string;
    none: string;
    unknown: string;
    loading: string;
    builtIn: string;
    custom: string;
    elevatedAdmin: string;
    standardUser: string;
    desktopConnected: string;
    browserPreviewTitle: string;
    browserPreviewBody: string;
    uacNotice: string;
    retry: string;
    adapterLoadErrorTitle: string;
    technicalDetails: string;
    copy: string;
    copied: string;
  };
  status: {
    connected: string;
    disconnected: string;
    error: string;
    connectedDetail: string;
    disconnectedDetail: string;
    errorDetail: string;
  };
  adapterStatus: {
    up: string;
    disconnected: string;
    disabled: string;
    notPresent: string;
    unknown: string;
  };
  adapterTypes: {
    wifi: string;
    ethernet: string;
    virtual: string;
    loopback: string;
    other: string;
  };
  dnsModes: {
    automatic: string;
    static: string;
    indeterminate: string;
  };
  dhcpStatuses: {
    enabled: string;
    disabled: string;
    unknown: string;
  };
  dashboard: {
    title: string;
    subtitle: string;
    connectionStatusLabel: string;
    activeViaApp: string;
    notConfiguredViaApp: string;
    dnsInfoTitle: string;
    selectedProvider: string;
    primaryDns: string;
    secondaryDns: string;
    activeAdapter: string;
    currentDnsMode: string;
    startButton: string;
    startingButton: string;
    stopButton: string;
    stoppingButton: string;
    startTooltip: string;
    stopTooltip: string;
    networkInfoTitle: string;
    adapterName: string;
    adapterType: string;
    adapterStatus: string;
    ipv4Address: string;
    gateway: string;
    currentDnsServers: string;
    dhcpStatus: string;
    operationInfoTitle: string;
    lastOperation: string;
    lastTimestamp: string;
    lastResult: string;
    errorDetail: string;
    operations: {
      apply: string;
      restore: string;
      refresh: string;
      none: string;
    };
    outcomes: {
      success: string;
      error: string;
      cancelled: string;
      idle: string;
    };
    backupSectionTitle: string;
    backupAdapter: string;
    backupTimestamp: string;
    backupOriginalMode: string;
    backupOriginalServers: string;
    backupAvailability: string;
    backupAvailable: string;
    backupUnavailable: string;
    backupCorrupted: string;
    discardCorruptBackup: string;
    dnsModeIndeterminateWarning: string;
    otherBackupsWarning: string;
    noAdapterSelected: string;
    goToAdapters: string;
  };
  providers: {
    title: string;
    subtitle: string;
    builtInSection: string;
    customSection: string;
    emptyCustom: string;
    manageCustomLink: string;
    selectedBanner: string;
    selectionNotice: string;
    detailsTitle: string;
    categoryLabel: string;
    primaryLabel: string;
    secondaryLabel: string;
    providerDescriptions: Record<string, string>;
    providerCategories: Record<string, string>;
  };
  customDns: {
    title: string;
    subtitle: string;
    formAddTitle: string;
    formEditTitle: string;
    nameLabel: string;
    namePlaceholder: string;
    primaryLabel: string;
    primaryPlaceholder: string;
    secondaryLabel: string;
    secondaryPlaceholder: string;
    descriptionLabel: string;
    descriptionPlaceholder: string;
    listTitle: string;
    emptyList: string;
    builtInProtectedNote: string;
    saveChanges: string;
    createProvider: string;
    cancelEdit: string;
    deleteConfirmTitle: string;
    deleteConfirmMessage: string;
    validation: {
      nameRequired: string;
      duplicateName: string;
      primaryInvalid: string;
      secondaryInvalid: string;
      secondarySameAsPrimary: string;
      ipv4Errors: Record<Ipv4ValidationErrorKey, string>;
    };
  };
  adapters: {
    title: string;
    subtitle: string;
    emptyList: string;
    selectAdapter: string;
    activeBadge: string;
    detailsTitle: string;
    guidLabel: string;
    indexLabel: string;
    macLabel: string;
    speedLabel: string;
    ipv4Label: string;
    gatewayLabel: string;
    dnsLabel: string;
    dnsModeLabel: string;
    dhcpLabel: string;
    disabledHint: string;
    disconnectedHint: string;
    selectionSafeNote: string;
  };
  settings: {
    title: string;
    subtitle: string;
    appearanceTitle: string;
    themeLabel: string;
    themeDark: string;
    themeLight: string;
    themeSystem: string;
    languageTitle: string;
    languageLabel: string;
    languageFa: string;
    languageEn: string;
    behaviorTitle: string;
    rememberAdapter: string;
    rememberAdapterDesc: string;
    rememberProvider: string;
    rememberProviderDesc: string;
    startPageLabel: string;
    confirmBeforeRestore: string;
    confirmBeforeRestoreDesc: string;
    showNotifications: string;
    showNotificationsDesc: string;
    savedNotice: string;
  };
  about: {
    title: string;
    subtitle: string;
    appNameLabel: string;
    versionLabel: string;
    descriptionBody: string;
    technologyTitle: string;
    developerTitle: string;
    developerPlaceholder: string;
    copyrightTitle: string;
    copyrightBody: string;
    securityTitle: string;
    securityBody: string;
    docsTitle: string;
    docsNote: string;
  };
  dialogs: {
    confirmApplyTitle: string;
    confirmApplyMessage: string;
    confirmApplySingleMessage: string;
    confirmRestoreTitle: string;
    confirmRestoreMessage: string;
    confirmDiscardBackupTitle: string;
    confirmDiscardBackupMessage: string;
  };
  notifications: {
    applySuccessTitle: string;
    applySuccessBody: string;
    restoreSuccessTitle: string;
    restoreSuccessBody: string;
    corruptBackupDiscardedTitle: string;
    corruptBackupDiscardedBody: string;
    customSavedTitle: string;
    customSavedBody: string;
    customUpdatedTitle: string;
    customUpdatedBody: string;
    customDeletedTitle: string;
    customDeletedBody: string;
    uacCancelledTitle: string;
    operationErrorTitle: string;
  };
  errors: Record<ErrorCode, string>;
}

export const fa: Translations = {
  appName: 'تغییردهنده DNS',
  appTagline: 'مدیریت امن تنظیمات DNS در ویندوز',
  nav: {
    dashboard: 'داشبورد',
    providers: 'سرویس‌دهنده‌ها',
    customDns: 'DNS سفارشی',
    adapters: 'کارت‌های شبکه',
    settings: 'تنظیمات',
    about: 'درباره برنامه',
  },
  common: {
    refresh: 'بروزرسانی',
    refreshing: 'در حال بروزرسانی...',
    cancel: 'انصراف',
    confirm: 'تایید',
    save: 'ذخیره',
    add: 'افزودن',
    edit: 'ویرایش',
    delete: 'حذف',
    close: 'بستن',
    select: 'انتخاب',
    selected: 'انتخاب شده',
    none: 'ندارد',
    unknown: 'نامشخص',
    loading: 'در حال بارگذاری...',
    builtIn: 'پیش‌فرض',
    custom: 'سفارشی',
    elevatedAdmin: 'دسترسی مدیر (Administrator)',
    standardUser: 'کاربر عادی (نیاز به تایید UAC هنگام تغییر)',
    desktopConnected: 'موتور بومی ویندوز متصل است',
    browserPreviewTitle: 'حالت پیش‌نمایش مرورگر',
    browserPreviewBody:
      'رابط کاربری خارج از محیط دسکتاپ Tauri اجرا شده است. برای شناسایی واقعی کارت‌های شبکه و اعمال DNS در ویندوز ۱۰ یا ۱۱، برنامه را با دستور npm run tauri dev یا فایل اجرایی ویندوز اجرا کنید.',
    uacNotice:
      'تغییر یا بازیابی DNS با پنجره کنترل حساب کاربری ویندوز (UAC) و بدون اجرای همیشگی کل برنامه با دسترسی مدیر انجام می‌شود.',
    retry: 'تلاش مجدد',
    adapterLoadErrorTitle: 'خطا در دریافت اطلاعات کارت‌های شبکه',
    technicalDetails: 'جزئیات فنی',
    copy: 'کپی',
    copied: 'کپی شد',
  },
  status: {
    connected: 'متصل',
    disconnected: 'قطع',
    error: 'خطا',
    connectedDetail: 'تنظیمات DNS این کارت شبکه توسط برنامه اعمال و تایید شده است.',
    disconnectedDetail: 'هیچ تغییر فعالی از طرف برنامه روی این کارت شبکه اعمال نشده است.',
    errorDetail: 'در آخرین عملیات یا بررسی وضعیت کارت شبکه و پشتیبان خطا رخ داده است.',
  },
  adapterStatus: {
    up: 'متصل و فعال',
    disconnected: 'کابل یا شبکه قطع است',
    disabled: 'غیرفعال',
    notPresent: 'ناموجود',
    unknown: 'نامشخص',
  },
  adapterTypes: {
    wifi: 'بی‌سیم (Wi-Fi)',
    ethernet: 'کابلی (Ethernet)',
    virtual: 'مجازی / VPN',
    loopback: 'حلقه داخلی',
    other: 'سایر',
  },
  dnsModes: {
    automatic: 'خودکار (DHCP)',
    static: 'دستی (Static)',
    indeterminate: 'نامشخص (نیازمند بررسی)',
  },
  dhcpStatuses: {
    enabled: 'فعال',
    disabled: 'غیرفعال',
    unknown: 'نامشخص',
  },
  dashboard: {
    title: 'داشبورد مدیریت DNS',
    subtitle: 'وضعیت واقعی کارت شبکه، تنظیمات DNS و بازیابی امن نسخه پشتیبان',
    connectionStatusLabel: 'وضعیت اتصال DNS',
    activeViaApp: 'پیکربندی فعال از طریق DNS Changer',
    notConfiguredViaApp: 'در حالت اصلی سیستم (بدون تغییر فعال برنامه)',
    dnsInfoTitle: 'اطلاعات DNS انتخاب‌شده',
    selectedProvider: 'سرویس‌دهنده انتخابی',
    primaryDns: 'DNS اول (Primary)',
    secondaryDns: 'DNS دوم (Secondary)',
    activeAdapter: 'کارت شبکه فعال',
    currentDnsMode: 'حالت فعلی DNS',
    startButton: 'شروع (START)',
    startingButton: 'در حال اعمال و تایید...',
    stopButton: 'توقف / بازیابی (STOP)',
    stoppingButton: 'در حال بازیابی...',
    startTooltip: 'اعمال سرویس‌دهنده انتخابی روی کارت شبکه پس از ذخیره پشتیبان',
    stopTooltip: 'بازگرداندن تنظیمات اصلی DNS از نسخه پشتیبان ذخیره شده',
    networkInfoTitle: 'مشخصات کارت شبکه',
    adapterName: 'نام کارت شبکه',
    adapterType: 'نوع رابط',
    adapterStatus: 'وضعیت رابط',
    ipv4Address: 'نشانی IPv4',
    gateway: 'دروازه پیش‌فرض (Gateway)',
    currentDnsServers: 'سرورهای DNS فعلی',
    dhcpStatus: 'وضعیت DHCP',
    operationInfoTitle: 'وضعیت آخرین عملیات',
    lastOperation: 'آخرین عملیات',
    lastTimestamp: 'زمان ثبت',
    lastResult: 'نتیجه',
    errorDetail: 'جزئیات خطا',
    operations: {
      apply: 'اعمال DNS',
      restore: 'بازیابی DNS اصلی',
      refresh: 'بروزرسانی شبکه',
      none: 'عملیاتی انجام نشده',
    },
    outcomes: {
      success: 'موفقیت‌آمیز و تایید شده',
      error: 'ناموفق',
      cancelled: 'لغو توسط کاربر (UAC)',
      idle: 'آماده',
    },
    backupSectionTitle: 'پشتیبان و بازیابی تنظیمات اصلی',
    backupAdapter: 'کارت شبکه مرتبط با پشتیبان',
    backupTimestamp: 'زمان ایجاد پشتیبان',
    backupOriginalMode: 'حالت اصلی DNS قبل از تغییر',
    backupOriginalServers: 'آدرس‌های اصلی DNS',
    backupAvailability: 'امکان بازیابی',
    backupAvailable: 'نسخه پشتیبان معتبر موجود است',
    backupUnavailable: 'نسخه پشتیبانی برای این کارت ثبت نشده است',
    backupCorrupted: 'فایل پشتیبان دچار خطای یکپارچگی شده و دست‌نخورده حفظ گردید',
    discardCorruptBackup: 'قرنطینه امن پشتیبان خراب',
    dnsModeIndeterminateWarning:
      'حالت فعلی DNS این کارت شبکه به صورت قطعی قابل تشخیص نیست (ممکن است وضعیت رجیستری و سیستم متعارض باشد). تا زمان رفع مشکل و بروزرسانی، دکمه شروع غیرفعال است.',
    otherBackupsWarning:
      'توجه: برای یک یا چند کارت شبکه دیگر نیز نسخه پشتیبان بازیابی‌نشده وجود دارد.',
    noAdapterSelected: 'هیچ کارت شبکه‌ای انتخاب نشده یا یافت نشد.',
    goToAdapters: 'مشاهده کارت‌های شبکه',
  },
  providers: {
    title: 'سرویس‌دهنده‌های DNS',
    subtitle: 'سرویس‌دهنده مورد نظر را انتخاب کنید؛ اعمال تغییر تنها با زدن دکمه شروع در داشبورد انجام می‌شود.',
    builtInSection: 'سرویس‌دهنده‌های استاندارد داخلی',
    customSection: 'سرویس‌دهنده‌های سفارشی شما',
    emptyCustom: 'هنوز سرویس‌دهنده سفارشی ثبت نشده است.',
    manageCustomLink: 'مدیریت DNS سفارشی',
    selectedBanner: 'سرویس‌دهنده فعال برای اعمال:',
    selectionNotice: 'انتخاب سرویس‌دهنده تنها وضعیت انتخابی را به‌روزرسانی می‌کند و تغییری در شبکه اعمال نمی‌کند.',
    detailsTitle: 'جزئیات سرویس‌دهنده انتخابی',
    categoryLabel: 'دسته‌بندی',
    primaryLabel: 'سرور اول',
    secondaryLabel: 'سرور دوم',
    providerDescriptions: {
      cloudflare: 'سرویس‌دهنده سریع و متمرکز بر حریم خصوصی Cloudflare.',
      google: 'سرویس DNS عمومی و پایدار گوگل با پوشش جهانی.',
      quad9: 'سرویس امنیتی Quad9 با مسدودسازی دامنه‌های مخرب.',
      opendns: 'سرویس قابل اعتماد Cisco OpenDNS برای وب‌گردی پایدار.',
      adguard: 'مسدودسازی تبلیغات و ردیاب‌ها در سطح DNS توسط AdGuard.',
      'cleanbrowsing-adult': 'فیلتر محتوای غیراخلاقی و دامنه‌های مخرب.',
      'cleanbrowsing-family': 'پالایش خانوادگی جهت مسدودسازی محتوای نامناسب و بدافزار.',
      'dns-watch': 'سرویس‌دهنده بدون سانسور و بدون ثبت گزارش فعالیت.',
    },
    providerCategories: {
      'Speed & Privacy': 'سرعت و حریم خصوصی',
      General: 'عمومی',
      Security: 'امنیت',
      Reliability: 'پایداری',
      'Ad Blocking': 'مسدودسازی تبلیغات',
      'Content Filter': 'پالایش محتوا',
      'Family Safety': 'ایمنی خانواده',
      Uncensored: 'بدون فیلتر',
      Custom: 'سفارشی',
    },
  },
  customDns: {
    title: 'مدیریت DNS سفارشی',
    subtitle: 'افزودن، ویرایش و حذف سرورهای IPv4 دلخواه با اعتبارسنجی دقیق در بک‌اند',
    formAddTitle: 'افزودن سرویس‌دهنده سفارشی جدید',
    formEditTitle: 'ویرایش سرویس‌دهنده سفارشی',
    nameLabel: 'نام سرویس‌دهنده',
    namePlaceholder: 'مثلاً: شبکه داخلی یا سرور اختصاصی',
    primaryLabel: 'DNS اول (IPv4)',
    primaryPlaceholder: 'مثلاً 1.1.1.1',
    secondaryLabel: 'DNS دوم (IPv4 · اختیاری)',
    secondaryPlaceholder: 'اختیاری — مثلاً 1.0.0.1',
    descriptionLabel: 'توضیحات (اختیاری)',
    descriptionPlaceholder: 'توضیح کوتاه درباره کاربرد این DNS',
    listTitle: 'فهرست سرویس‌دهنده‌های سفارشی ذخیره‌شده',
    emptyList: 'هیچ سرویس‌دهنده سفارشی ذخیره نشده است.',
    builtInProtectedNote: 'سرویس‌دهنده‌های داخلی برنامه قابل ویرایش یا حذف نیستند.',
    saveChanges: 'ذخیره تغییرات',
    createProvider: 'افزودن سرویس‌دهنده',
    cancelEdit: 'لغو ویرایش',
    deleteConfirmTitle: 'حذف سرویس‌دهنده سفارشی',
    deleteConfirmMessage: 'آیا از حذف سرویس‌دهنده «{name}» اطمینان دارید؟ این عملیات قابل بازگشت نیست.',
    validation: {
      nameRequired: 'وارد کردن نام سرویس‌دهنده الزامی است.',
      duplicateName: 'سرویس‌دهنده‌ای با این نام از قبل وجود دارد.',
      primaryInvalid: 'آدرس DNS اول نامعتبر است: ',
      secondaryInvalid: 'آدرس DNS دوم نامعتبر است: ',
      secondarySameAsPrimary: 'آدرس DNS دوم نمی‌تواند با DNS اول یکسان باشد.',
      ipv4Errors: {
        emptyOrWhitespace: 'نشانی نباید خالی یا دارای فاصله اضافی باشد.',
        invalidFormat: 'باید یک نشانی معتبر چهاربخشی IPv4 باشد (مانند 1.1.1.1).',
        leadingZeros: 'استفاده از صفر ابتدایی در بخش‌های IPv4 مجاز نیست.',
        octetOutOfRange: 'هر بخش IPv4 باید عددی بین ۰ تا ۲۵۵ باشد.',
        unspecifiedAddress: 'نشانی 0.0.0.0 به عنوان سرور DNS مجاز نیست.',
        broadcastAddress: 'نشانی broadcast 255.255.255.255 مجاز نیست.',
        multicastAddress: 'نشانی‌های Multicast برای سرور DNS مجاز نیستند.',
      },
    },
  },
  adapters: {
    title: 'کارت‌های شبکه ویندوز',
    subtitle: 'کارت‌های شبکه شناسایی‌شده بر اساس شناسه پایدار GUID؛ انتخاب کارت شبکه تغییری در تنظیمات آن ایجاد نمی‌کند.',
    emptyList: 'هیچ کارت شبکه‌ای شناسایی نشد.',
    selectAdapter: 'انتخاب کارت شبکه',
    activeBadge: 'کارت شبکه انتخاب‌شده',
    detailsTitle: 'جزئیات کامل کارت شبکه انتخابی',
    guidLabel: 'شناسه پایدار (GUID)',
    indexLabel: 'شماره رابط (Interface Index)',
    macLabel: 'نشانی MAC',
    speedLabel: 'سرعت لینک',
    ipv4Label: 'نشانی‌های IPv4',
    gatewayLabel: 'دروازه پیش‌فرض',
    dnsLabel: 'DNS های IPv4 فعلی',
    dnsModeLabel: 'حالت DNS',
    dhcpLabel: 'وضعیت DHCP',
    disabledHint: 'این کارت شبکه غیرفعال است و تا زمان فعال‌سازی در ویندوز امکان تغییر DNS ندارد.',
    disconnectedHint: 'این کارت شبکه به شبکه متصل نیست.',
    selectionSafeNote: 'تغییر کارت شبکه انتخابی هیچ تنظیماتی را روی کارت‌های دیگر تغییر نمی‌دهد.',
  },
  settings: {
    title: 'تنظیمات برنامه',
    subtitle: 'شخصی‌سازی ظاهر، زبان و رفتار ذخیره‌سازی برنامه',
    appearanceTitle: 'ظاهر و پوسته',
    themeLabel: 'پوسته رنگی برنامه',
    themeDark: 'تیره (Dark)',
    themeLight: 'روشن (Light)',
    themeSystem: 'هماهنگ با سیستم',
    languageTitle: 'زبان رابط کاربری',
    languageLabel: 'انتخاب زبان',
    languageFa: 'فارسی (RTL)',
    languageEn: 'English (LTR)',
    behaviorTitle: 'رفتار و ترجیحات برنامه',
    rememberAdapter: 'به خاطر سپردن آخرین کارت شبکه انتخابی',
    rememberAdapterDesc: 'هنگام اجرای مجدد برنامه، کارت شبکه انتخابی قبلی به صورت خودکار انتخاب شود.',
    rememberProvider: 'به خاطر سپردن آخرین سرویس‌دهنده انتخابی',
    rememberProviderDesc: 'انتخاب سرویس‌دهنده DNS در فایل تنظیمات محلی ذخیره شود.',
    startPageLabel: 'صفحه شروع هنگام باز شدن برنامه',
    confirmBeforeRestore: 'درخواست تایید پیش از بازیابی DNS اصلی',
    confirmBeforeRestoreDesc: 'پیش از بازگرداندن تنظیمات پشتیبان، پنجره تایید نمایش داده شود.',
    showNotifications: 'نمایش اعلان پس از عملیات موفق',
    showNotificationsDesc: 'پس از تایید اعمال یا بازیابی DNS اعلان تایید نمایش داده شود.',
    savedNotice: 'تنظیمات با موفقیت در حافظه محلی ذخیره شد.',
  },
  about: {
    title: 'درباره برنامه',
    subtitle: 'مشخصات فنی، معماری و وضعیت امنیتی DNS Changer',
    appNameLabel: 'نام برنامه',
    versionLabel: 'نسخه',
    descriptionBody:
      'DNS Changer یک برنامه دسکتاپ بومی برای ویندوز ۱۰ و ۱۱ است که تغییر، پشتیبان‌گیری، اعتبارسنجی و بازیابی تنظیمات DNS نسخه IPv4 کارت‌های شبکه را به صورت امن و قابل اعتماد انجام می‌دهد.',
    technologyTitle: 'فناوری‌های به‌کاررفته',
    developerTitle: 'اطلاعات توسعه‌دهنده',
    developerPlaceholder: 'تیم توسعه DNS Changer (بخش اطلاعات توسعه‌دهنده قابل تکمیل در نسخه انتشار نهایی)',
    copyrightTitle: 'حقوق و مالکیت معنوی',
    copyrightBody: 'کپی‌رایت © ۲۰۲۶ پروژه DNS Changer. تمامی حقوق محفوظ است.',
    securityTitle: 'اصول امنیت و پایداری',
    securityBody:
      'تمامی عملیات شبکه‌ای با اعتبارسنجی سخت‌گیرانه در Rust، اجرای کنترل‌شده و غیرتعاملی دستورات ویندوز، تفکیک پشتیبان‌ها بر اساس GUID، و بررسی یکپارچگی SHA-256 انجام می‌شود.',
    docsTitle: 'مستندات پروژه',
    docsNote: 'راهنمای کامل توسعه، تست و ساخت فایل نصبی ویندوز در فایل README.md مخزن پروژه موجود است.',
  },
  dialogs: {
    confirmApplyTitle: 'اعمال تنظیمات DNS',
    confirmApplyMessage:
      'آیا می‌خواهید DNS کارت شبکه «{adapter}» به «{provider}» ({primary} / {secondary}) تغییر یابد؟ نسخه پشتیبان تنظیمات فعلی پیش از تغییر ذخیره می‌شود.',
    confirmApplySingleMessage:
      'آیا می‌خواهید DNS کارت شبکه «{adapter}» به «{provider}» ({primary}) تغییر یابد؟ نسخه پشتیبان تنظیمات فعلی پیش از تغییر ذخیره می‌شود.',
    confirmRestoreTitle: 'بازیابی تنظیمات اصلی DNS',
    confirmRestoreMessage:
      'آیا از بازگرداندن تنظیمات اصلی DNS کارت شبکه «{adapter}» از نسخه پشتیبان اطمینان دارید؟',
    confirmDiscardBackupTitle: 'قرنطینه فایل پشتیبان خراب',
    confirmDiscardBackupMessage:
      'فایل پشتیبان کارت شبکه «{adapter}» دچار خطای یکپارچگی است و مانع اعمال یا بازیابی می‌شود. با تایید، فایل به صورت امن به پسوند .corrupt تغییرنام داده و کنار گذاشته می‌شود (بدون حذف دائمی) تا بتوانید دوباره روی این کارت شبکه عملیات انجام دهید. ادامه می‌دهید؟',
  },
  notifications: {
    applySuccessTitle: 'تنظیمات DNS اعمال و تایید شد',
    applySuccessBody: 'سرورهای {servers} با موفقیت روی کارت شبکه «{adapter}» تنظیم شدند.',
    restoreSuccessTitle: 'تنظیمات اصلی DNS بازیابی شد',
    restoreSuccessBody: 'کارت شبکه «{adapter}» با موفقیت به حالت اصلی بازگردانده شد.',
    corruptBackupDiscardedTitle: 'فایل پشتیبان خراب قرنطینه شد',
    corruptBackupDiscardedBody: 'فایل پشتیبان نامعتبر کارت شبکه «{adapter}» به صورت امن کنار گذاشته شد.',
    customSavedTitle: 'سرویس‌دهنده سفارشی ثبت شد',
    customSavedBody: '«{name}» به فهرست سرویس‌دهنده‌های سفارشی اضافه شد.',
    customUpdatedTitle: 'سرویس‌دهنده سفارشی ویرایش شد',
    customUpdatedBody: 'تغییرات «{name}» با موفقیت ذخیره شد.',
    customDeletedTitle: 'سرویس‌دهنده سفارشی حذف شد',
    customDeletedBody: 'سرویس‌دهنده از فهرست محلی حذف گردید.',
    uacCancelledTitle: 'درخواست مجوز مدیر لغو شد',
    operationErrorTitle: 'خطا در اجرای عملیات',
  },
  errors: {
    ADAPTER_NOT_FOUND: 'کارت شبکه انتخاب‌شده در ویندوز یافت نشد. فهرست را بروزرسانی کنید.',
    ADAPTER_DISABLED: 'کارت شبکه انتخاب‌شده غیرفعال است. ابتدا آن را در ویندوز فعال کنید.',
    ADAPTER_DISCONNECTED: 'کارت شبکه به شبکه متصل نیست. پس از اتصال دوباره تلاش کنید.',
    INVALID_DNS_ADDRESS: 'نشانی IPv4 واردشده برای DNS معتبر نیست.',
    INVALID_ADAPTER_GUID: 'شناسه GUID کارت شبکه معتبر نیست.',
    ADMIN_ACCESS_DENIED: 'دسترسی مدیر (Administrator) تایید نشد. تغییری اعمال نگردید.',
    UAC_CANCELLED: 'پنجره تایید دسترسی مدیر (UAC) توسط کاربر لغو شد. تنظیمی تغییر نکرد.',
    ELEVATION_FAILED: 'اجرای فرآیند کمکی با دسترسی مدیر ناموفق بود.',
    POWERSHELL_EXECUTION_FAILED: 'اجرای دستور شبکه‌ای ویندوز با خطا مواجه شد.',
    POWERSHELL_BLOCKED:
      'اجرای PowerShell توسط آنتی‌ویروس، سیاست امنیتی یا نبود یکی از قابلیت‌های ویندوز مسدود شده است. لطفاً آنتی‌ویروس و ویژگی‌های ویندوز (Windows Features) را بررسی کنید.',
    POWERSHELL_TIMEOUT: 'زمان اجرای دستور شبکه‌ای ویندوز به پایان رسید.',
    DNS_APPLY_FAILED: 'اعمال DNS جدید ناموفق بود. نسخه پشتیبان اصلی حفظ شده است.',
    DNS_MODE_INDETERMINATE:
      'حالت فعلی DNS این کارت شبکه به صورت قطعی قابل تشخیص نبود؛ برای حفظ ایمنی تنظیمات تغییری اعمال نشد.',
    IPV6_MODIFICATION_DETECTED:
      'تغییر غیرمنتظره در DNS نسخه IPv6 شناسایی شد؛ بازیابی IPv6 انجام شد و پشتیبان اصلی IPv4 حفظ گردید.',
    IPV6_RECOVERY_FAILED:
      'تغییر غیرمنتظره در DNS نسخه IPv6 رخ داد؛ تنظیمات IPv4 بازگردانده شد اما بازیابی خودکار IPv6 ناموفق بود. لطفاً تنظیمات IPv6 کارت شبکه را در ویندوز بررسی کنید.',
    VERIFICATION_FAILED: 'تنظیمات پس از اعمال با مقادیر درخواستی مطابقت نداشت. پشتیبان حفظ شد.',
    ROLLBACK_NOT_PERFORMED:
      'تنظیمات DNS تغییر کرد اما بازگشت خودکار به حالت قبلی انجام نشد. لطفاً وضعیت کارت شبکه را بررسی کرده یا از دکمه توقف/بازیابی استفاده کنید؛ پشتیبان اصلی حفظ شده است.',
    BACKUP_CORRUPTION: 'فایل پشتیبان دچار مشکل یکپارچگی است و برای جلوگیری از آسیب بازنویسی نشد.',
    BACKUP_NOT_FOUND: 'نسخه پشتیبانی برای این کارت شبکه یافت نشد.',
    RESTORE_FAILED: 'بازیابی تنظیمات اصلی ناموفق بود. فایل پشتیبان همچنان محفوظ است.',
    NETWORK_CHANGED: 'وضعیت کارت شبکه هنگام اجرای عملیات تغییر کرد. لطفاً بروزرسانی کنید.',
    CONCURRENT_OPERATION: 'عملیات دیگری روی همین کارت شبکه در حال اجراست. لطفاً صبر کنید.',
    DUPLICATE_PROVIDER_NAME: 'سرویس‌دهنده‌ای با این نام از قبل وجود دارد.',
    INVALID_INPUT: 'مقادیر ورودی نامعتبر است. لطفاً ورودی‌ها را بررسی کنید.',
    UNSUPPORTED_PLATFORM: 'تغییر واقعی DNS تنها در نسخه دسکتاپ ویندوز ۱۰ و ۱۱ پشتیبانی می‌شود.',
    STORAGE_ERROR: 'خطا در خواندن یا نوشتن اطلاعات ذخیره‌سازی محلی.',
    INTERNAL_ERROR: 'خطای داخلی غیرمنتظره رخ داد.',
  },
};
