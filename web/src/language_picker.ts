import { getLanguage, setLanguage } from './i18n';

export function createLanguagePicker(root: HTMLElement, onChange: () => void) {
    const trigger = root.querySelector<HTMLButtonElement>('#language')!;
    const value = root.querySelector<HTMLElement>('#language-value')!;
    const menu = root.querySelector<HTMLElement>('#language-menu')!;
    const options = Array.from(menu.querySelectorAll<HTMLButtonElement>('[data-language]'));

    function sync() {
        const language = getLanguage();
        value.textContent = language === 'ru' ? 'Русский' : 'English';
        value.lang = language;
        options.forEach(option => option.setAttribute('aria-checked', String(option.dataset.language === language)));
    }

    function close(returnFocus = false) {
        menu.hidden = true;
        trigger.setAttribute('aria-expanded', 'false');
        if (returnFocus) trigger.focus();
    }

    function open() {
        menu.hidden = false;
        trigger.setAttribute('aria-expanded', 'true');
        options.find(option => option.dataset.language === getLanguage())?.focus();
    }

    trigger.onclick = () => { if (menu.hidden) open(); else close(); };
    trigger.onkeydown = event => {
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            event.preventDefault();
            open();
        }
    };
    options.forEach(option => {
        option.onclick = () => {
            setLanguage(option.dataset.language === 'ru' ? 'ru' : 'en');
            onChange();
            close(true);
        };
    });
    menu.onkeydown = event => {
        const index = options.indexOf(document.activeElement as HTMLButtonElement);
        switch (event.key) {
            case 'ArrowDown':
            case 'ArrowUp': {
                event.preventDefault();
                const direction = event.key === 'ArrowDown' ? 1 : -1;
                options[(index + direction + options.length) % options.length].focus();
                break;
            }
            case 'Home':
            case 'End':
                event.preventDefault();
                options[event.key === 'Home' ? 0 : options.length - 1].focus();
                break;
            case 'Escape':
                event.preventDefault();
                close(true);
                break;
            case 'Tab':
                close(true);
                break;
        }
    };
    document.addEventListener('pointerdown', event => {
        if (event.target instanceof Node && !root.contains(event.target)) close();
    });
    root.addEventListener('focusout', event => {
        if (!(event.relatedTarget instanceof Node) || !root.contains(event.relatedTarget)) close();
    });

    return { sync };
}
