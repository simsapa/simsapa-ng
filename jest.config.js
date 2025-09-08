module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'jsdom',
  roots: ['<rootDir>/src-ts'],
  testMatch: ['**/*.test.ts'],
  collectCoverageFrom: [
    'src-ts/**/*.ts',
    '!src-ts/**/*.test.ts',
  ],
  setupFilesAfterEnv: ['<rootDir>/src-ts/test-setup.ts']
};