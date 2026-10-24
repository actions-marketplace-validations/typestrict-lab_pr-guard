chore: add .prettierrc and run prettier

We had `npm run format` but no config, so everyone's editor used different widths. Pins
printWidth to 80 and reformats the existing files. Formatting only.
