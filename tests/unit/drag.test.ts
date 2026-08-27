import test from 'node:test';
import assert from 'node:assert/strict';
import { exceedsDragThreshold } from '../../src/shared/drag';

test('drag begins only at the four-pixel movement threshold', () => {
  assert.equal(exceedsDragThreshold({ x: 10, y: 10 }, { x: 13, y: 10 }), false);
  assert.equal(exceedsDragThreshold({ x: 10, y: 10 }, { x: 14, y: 10 }), true);
  assert.equal(exceedsDragThreshold({ x: 0, y: 0 }, { x: 3, y: 3 }), true);
});
