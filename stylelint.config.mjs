export default {
  extends: ["stylelint-config-standard", "stylelint-config-recess-order"],
  rules: {
    "hue-degree-notation": null,
    "import-notation": null,
    "lightness-notation": null,
    "at-rule-no-unknown": [
      true,
      {
        ignoreAtRules: [
          "apply",
          "custom-variant",
          "plugin",
          "reference",
          "source",
          "tailwind",
          "theme",
          "utility",
          "variant",
        ],
      },
    ],
  },
};
