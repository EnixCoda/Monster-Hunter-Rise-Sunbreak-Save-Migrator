import base from './playwright.config.js';
export default {
  ...base,
  use: { ...base.use, video: 'on' },
};
