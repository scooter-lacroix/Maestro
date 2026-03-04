[![npm version](https://img.shields.io/npm/v/unique-username-generator)](https://www.npmjs.com/package/unique-username-generator)
[![downloads](https://img.shields.io/npm/dw/unique-username-generator)](https://www.npmjs.com/package/unique-username-generator)
[![license: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![TypeScript types](https://img.shields.io/badge/TS-Types%20included-3178c6?logo=typescript)](https://www.npmjs.com/package/unique-username-generator)
[![install size](https://packagephobia.com/badge?p=unique-username-generator)](https://packagephobia.com/result?p=unique-username-generator)
[![bundle size](https://deno.bundlejs.com/badge?q=unique-username-generator@latest)](https://bundlejs.com/?q=unique-username-generator@latest)
[![install size](https://img.shields.io/packagephobia/install/unique-username-generator)](https://packagephobia.com/result?p=unique-username-generator)
[![semantic-release](https://img.shields.io/badge/release-semantic--release-e10079.svg)](https://github.com/semantic-release/semantic-release)

![Visitors](https://api.visitorbadge.io/api/combined?path=https%3A%2F%2Fwww.npmjs.com%2Fpackage%2Funique-username-generator&countColor=%23dce775&style=flat)
<span class="badge-buymeacoffee">
<a href="https://www.buymeacoffee.com/subhamg" title="Donate to this project using Buy Me A Coffee"><img src="https://img.shields.io/badge/buy%20me%20a%20coffee-donate-yellow.svg" alt="Buy Me A Coffee donate button" /></a>
</span>

# unique-username-generator

A tiny, flexible username generator for Node and browsers. Generate from email or dictionaries; control separator, style, max length, optional digits, profanity filtering, templates, deterministic seeds, and batch generation.

Security note: this library generates human-friendly display names. It is not intended for security-sensitive randomness (e.g., passwords, tokens). It prefers Web Crypto when available and falls back to a non-crypto PRNG only as a last resort.

[![NPM](https://nodei.co/npm/unique-username-generator.svg)](https://nodei.co/npm/unique-username-generator/)

## Installation

```bash
npm install unique-username-generator --save
```

- Importing

```javascript
// Using Node.js `require()`
const { generateFromEmail, generateUsername } = require("unique-username-generator");
// Using ES6 imports
import { generateFromEmail, generateUsername } from "unique-username-generator";
```

## Usage

### Generate username from email

It will generate username from email and add upto six random digits at the end of the name.

```javascript
// Simple: add three random digits
generateFromEmail("lakshmi.narayan@example.com", 3); // "lakshminarayan234"

// Advanced: options
generateFromEmail("123john@example.com", { randomDigits: 2, stripLeadingDigits: true }); // "john12"
generateFromEmail("12345@example.com", { randomDigits: 0, leadingFallback: "member" });  // "member"
```

### Randomly generate unique username.

It will generate unique username from adjectives, nouns, random digits and separator. You can control these following parameters - separator, number of random digits and maximum length of a username.

```javascript
// generaterUsername(separator, number of random digits, maximum length)

// Without any parameter
const username = generateUsername();
console.log(username); // blossomlogistical

// With any separator like "-, _"
const username = generateUsername("-");
console.log(username); // blossom-logistical

// With random digits and no separator
const username = generateUsername("", 3);
console.log(username); // blossomlogistical732

// With maximum length constraint and no separator, no random digits
const username = generateUsername("", 0, 15);
console.log(username); // blossomlogistic

// With maximum length constraint and separator but no random digits
const username = generateUsername("-", 0, 15);
console.log(username); // blossom-logisti

// With maximum length constraint and random digits but no separator
const username = generateUsername("", 2, 19);
console.log(username); // blossomlogistical73

// With all parameters
const username = generateUsername("-", 2, 20, "unique username");
console.log(username); // unique-username-73
```

### Default dictionaries

By default, the unique username generator library comes with 2 dictionaries out of the box, so that you can use them straight away.

The new syntax for using the default dictionaries is the following:

```javascript
import { uniqueUsernameGenerator, Config, adjectives, nouns } from 'unique-username-generator';

const config: Config = {
  dictionaries: [adjectives, nouns]
}

const username: string = uniqueUsernameGenerator(config); // blossomlogistical
```

### Custom dictionaries

You might want to provide your custom dictionaries to use for generating your unique username, in order to meet your project requirements. You can easily do that using the dictionaries option.

```javascript
import { uniqueUsernameGenerator } from 'unique-username-generator';

const marvelCharacters = [
  'Iron Man',
  'Doctor Strange',
  'Hulk',
  'Captain America',
  'Thanos'
];

const config: Config = {
  dictionaries: [marvelCharacters],
  separator: '',
  style: 'capital',
  randomDigits: 3
}

const username: string = uniqueUsernameGenerator(config); // Hulk123
```

### Profanity filtering and exclusions

By default, the generator filters out common profane words from the built-in dictionaries to avoid unsafe names. You can extend or override the blocklist using `exclude` and `profanityList`.

```ts
import { uniqueUsernameGenerator, adjectives, nouns, DEFAULT_PROFANITY } from 'unique-username-generator';

const username = uniqueUsernameGenerator({
  dictionaries: [adjectives, nouns],
  exclude: ['beta', 'foo'],            // custom exclusions
  profanityList: DEFAULT_PROFANITY,    // extend/override blocklist
  randomDigits: 0
});
```

### Output styles

Additional styles are supported beyond `lowerCase`, `upperCase`, and `capital`:
- `camelCase`
- `pascalCase`
- `kebabCase`
- `snakeCase`
- `titleCase` (capitalize each word)

```ts
uniqueUsernameGenerator({ dictionaries: [["blue"],["whale"]], separator: "-", style: 'camelCase', randomDigits: 0 }); // blueWhale
```

### Templates and deterministic output

```ts
// Template tokens: {adjective}, {noun}, positional {0},{1},... and {digits:n}
uniqueUsernameGenerator({
  dictionaries: [adjectives, nouns],
  template: "{adjective}-{noun}-{digits:2}",
  seed: "v1",           // make output deterministic
  style: "lowerCase",
}); // e.g. "brave-otter-42"
```

### Batch generation

```ts
import { generateMany, generateUniqueAsync } from 'unique-username-generator';

// Generate N (optionally unique) usernames in-memory
const many = generateMany({ dictionaries: [adjectives, nouns], count: 5, unique: true });

// Generate a username that is unique against an external store
const taken = new Set(["cool-fox"]);
const username = await generateUniqueAsync(
  { dictionaries: [adjectives, nouns], separator: "-" },
  (candidate) => taken.has(candidate)
);
```

### CLI

After installing, a simple CLI is available as `usergen` (aliases: `usernamegen`, `unique-username`, `uuname`).

```
Usage: usergen [options]

Options:
  -s, --separator <sep>     Separator between words (default: empty)
  -d, --digits <n>          Number of random digits to append (0-6)
  -l, --length <n>          Maximum username length (default: 15)
      --style <style>       Style: lowerCase | upperCase | capital | camelCase | pascalCase | kebabCase | snakeCase | titleCase
  -U, --upper               Shortcut for --style upperCase
      --seed <seed>         Deterministic seed for reproducible output
  -t, --template <tpl>      Template, e.g. "{adjective}-{noun}-{digits:2}"
  -c, --count <n>           Generate many
  -u, --unique              Ensure unique usernames within this run
  -o, --out <file>          Write results to a file (UTF-8)
      --unsafe              Disable profanity filtering
  -U, --upper               Shortcut for --style upperCase
  -D, --dict <a,b,c>        Provide a custom dictionary (can be used multiple times)
  -x, --exclude <a,b,c>     Extra words to exclude
  -h, --help                Show help
```

## API

### uniqueUsernameGenerator (options)
Returns a `string` with a random generated username

### options

Type: `Config`

The `options` argument mostly corresponds to the properties defined for uniqueUsernameGenerator. Only `dictionaries` is required.


| Option       | Type                                | Description                                                                                                                                                                                                                                                                         | Default value | Example value                                                                                                                                                                                                           |
|--------------|-------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|---------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| dictionaries | `array`                             | This is an array of dictionaries. Each dictionary is an array of strings containing the words to use for generating the string.<br>The provided dictionaries can be imported from the library as a separate modules and provided in the desired order.                              | n/a           | <br>```import { uniqueUsernameGenerator, adjectives, nouns } from 'unique-username-generator';```<br>```const username: string = uniqueUsernameGenerator({ dictionaries: [nouns, adjectives]}); // blossomlogistical``` |
| separator    | `string`                            | A string separator to be used for separate the words generated. The default separator is set to be empty string.                                                                                                                                                                    | ""            | `-`                                                                                                                                                                                                                     |
| randomDigits | `number`                            | A number of random digits to add at the end of a username.                                                                                                                                                                                                                          | 0             | `3`                                                                                                                                                                                                                     |
| length       | `number`                            | A maximum length of a username                                                                                                                                                                                                                                                      | 15            | `12`                                                                                                                                                                                                                    |
| style        | `lowerCase \| upperCase \| capital \| titleCase` | The default value is set to `lowerCase` and it will return a lower case username.<br>By setting the value to `upperCase`, the words will be returned in upper case.<br>The `capital` option will capitalize only the first character of the full username.<br>`titleCase` will capitalize each word (token). | lowerCase     | `lowerCase` |

Additional options:

| Option          | Type         | Description | Default |
|-----------------|--------------|-------------|---------|
| exclude         | `string[]`   | Extra words to filter out | [] |
| profanityList   | `string[]`   | Profanity blocklist to apply to dictionaries | built-in minimal list |
| profanityOptions| `{ matchSubstrings?: boolean, wordBoundary?: string }` | Fine-tune matching | word-boundary matching |
| style           | extend to: `camelCase \| pascalCase \| kebabCase \| snakeCase \| titleCase` | Additional output styles | `lowerCase` |

## License

The MIT License.

## Thank you
If you'd like to say thanks, I'd really appreciate a coffee :)

<a href="https://www.buymeacoffee.com/subhamg" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" style="height: 50px !important;width: 200px !important;" ></a>