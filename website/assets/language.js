let savedLanguage;
try {
  savedLanguage = localStorage.getItem('ossia-site-language');
} catch {}
const browserLanguages = navigator.languages?.length
  ? navigator.languages
  : [navigator.language || ''];
document.documentElement.lang =
  savedLanguage === 'zh' || savedLanguage === 'en'
    ? savedLanguage
    : browserLanguages
        .map((x) => x.toLowerCase().split('-')[0])
        .find((x) => x === 'zh' || x === 'en') || 'en';

if (document.documentElement.lang === 'zh') document.documentElement.dataset.translating = '';
