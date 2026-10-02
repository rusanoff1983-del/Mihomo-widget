// UI strings only. Proxy/group/node names always stay as provided by Mihomo.
window.WidgetI18n = (() => {
  const en = {
    'Не подключено': 'Not connected', 'Проверить задержку': 'Check latency', 'Обновить': 'Refresh',
    'Настройки': 'Settings', 'Скрыть в трей': 'Hide to tray', 'Группы прокси': 'Proxy groups',
    'Настроить вкладки групп': 'Configure group tabs', 'Настроить группы': 'Configure groups',
    'Текущий узел': 'Current node', 'Подключение': 'Connection', 'Подключите Mihomo': 'Connect Mihomo',
    'Внешний IP': 'External IP', 'Скорость трафика': 'Traffic speed', 'Загрузка…': 'Loading…',
    'Назад': 'Back', 'Вкладки групп': 'Group tabs', 'Выберите, что показывать в панели': 'Choose what to show in the panel',
    'Отметьте группы для верхних вкладок. Стрелками меняйте их порядок, кнопкой «Назад» вернитесь к панели.': 'Select groups for the top tabs. Use the arrows to reorder them, then return to the panel.',
    'Подключение и оформление виджета': 'Connection and appearance', 'Подключение Mihomo': 'Mihomo connection',
    'Адрес контроллера': 'Controller address', 'Секрет / API Token': 'Secret / API token', 'секрет': 'secret',
    'Сохранить': 'Save', 'Проверить': 'Test',
    'Поддерживается любая Mihomo / Clash API панель (роутер, Clash Verge, FlClash и др.).': 'Works with any Mihomo / Clash API controller (router, Clash Verge, FlClash, etc.).',
    'Режим Mihomo': 'Mihomo mode', 'Язык': 'Language', 'Как в системе': 'System', 'Русский': 'Russian',
    'Тема приложения': 'Theme', 'Оформление': 'Appearance', 'Графит': 'Graphite', 'Светлая': 'Light',
    'Какао': 'Cocoa', 'Шалфей': 'Sage', 'Прозрачность фона': 'Background transparency',
    'Карточки и фон станут прозрачнее, текст останется чётким.': 'Cards and background become more transparent while text stays sharp.',
    'Карточки узлов': 'Node cards', 'Показывать флаги': 'Show flags',
    'Страна рядом с названием узла, если её удалось определить': 'Show the country beside a node when it can be detected',
    'Цветной пинг': 'Colored latency', 'Зелёный, янтарный и красный по задержке': 'Green, amber and red by latency',
    'Поведение окна': 'Window behavior', 'Поверх всех окон': 'Always on top',
    'Оставлять виджет поверх других приложений': 'Keep the widget above other apps',
    'Прилипать к краям': 'Snap to edges', 'Магнит к краям экрана и панели задач': 'Snap to screen and taskbar edges',
    'Клик насквозь': 'Click-through', 'Выключается через меню трея': 'Turn off from the tray menu',
    'Положение и размер окна сохраняются автоматически. Ctrl+Shift+Space — показать или скрыть виджет.': 'Window position and size are saved automatically. Ctrl+Shift+Space shows or hides the widget.',
    'Система и данные': 'System and data', 'Запускать вместе с Windows': 'Start with Windows',
    'Автозагрузка при входе в систему': 'Launch when you sign in',
    'Обновления': 'Updates', 'Проверка выпусков GitHub после публикации первой версии.': 'GitHub updates will be available after the first release.',
    'Проверить обновления': 'Check for updates', 'Установить обновление': 'Install update',
    'Укажите адрес Mihomo и API secret, чтобы начать.': 'Enter your Mihomo address and API secret to get started.',
    'Настроить подключение': 'Set up connection', 'Нет связи с панелью': 'Cannot reach controller',
    'Панель недоступна': 'Controller unavailable', 'Группа не выбрана': 'No group selected', 'Группы': 'Groups',
    'Переключение…': 'Switching…', 'Не удалось переключить': 'Could not switch',
    'В группе нет узлов': 'No nodes in this group', 'Узлы не ответили на проверку': 'Nodes did not respond',
    'Ошибка пинга': 'Latency check failed', 'Список пуст — панель недоступна.': 'List is empty — controller unavailable.',
    'Проверяю…': 'Checking…', 'Укажи адрес панели и API secret.': 'Enter controller address and API secret.',
    'панель отвечает': 'controller responded', 'Не отвечает. Проверь адрес и секрет.': 'No response. Check the address and secret.',
    'Панель не отвечает': 'Controller did not respond', 'Подключение обновлено': 'Connection updated',
    'Ошибка смены режима': 'Could not change mode', 'Пинг: {count} из {total} узлов': 'Latency: {count} of {total} nodes',
    'Режим: {mode}': 'Mode: {mode}', '{count} узл.': '{count} nodes', 'Поднять {name}': 'Move {name} up',
    'Опустить {name}': 'Move {name} down', 'Внешний IP: {ip} (нажмите для копирования)': 'External IP: {ip} (click to copy)',
    'Не удалось изменить настройку': 'Could not change setting',
    'Выпусков пока нет': 'No releases yet', 'Проверка обновлений…': 'Checking for updates…',
    'Обновлений нет': 'No updates available', 'Доступна версия {version}': 'Version {version} is available',
    'Не удалось проверить обновления': 'Could not check for updates',
  };
  const originals = new WeakMap();
  const attributeOriginals = new WeakMap();
  let language = 'ru';
  function resolve(preference) {
    return preference === 'system' ? (/^ru\b/i.test(navigator.language) ? 'ru' : 'en') : (preference === 'en' ? 'en' : 'ru');
  }
  function t(text, values = {}) {
    const source = language === 'en' ? (en[text] || text) : text;
    return source.replace(/\{(\w+)\}/g, (_, key) => String(values[key] ?? ''));
  }
  function apply(preference) {
    language = resolve(preference);
    document.documentElement.lang = language;
    const root = document.getElementById('widget-root');
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const node = walker.currentNode;
      if (!originals.has(node)) originals.set(node, node.nodeValue);
      const original = originals.get(node);
      const trimmed = original.trim();
      if (en[trimmed]) node.nodeValue = original.replace(trimmed, t(trimmed));
    }
    root.querySelectorAll('[title],[aria-label],[placeholder]').forEach(el => {
      if (!attributeOriginals.has(el)) {
        attributeOriginals.set(el, Object.fromEntries(['title', 'aria-label', 'placeholder'].map(a => [a, el.getAttribute(a)])));
      }
      for (const [attr, original] of Object.entries(attributeOriginals.get(el))) {
        if (original && en[original]) el.setAttribute(attr, t(original));
      }
    });
  }
  return { t, apply, resolve, get language() { return language; } };
})();
