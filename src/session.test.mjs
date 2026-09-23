import assert from "node:assert/strict";
import test from "node:test";
import {activeElapsed,resumeElapsed,TypingState} from "./session.ts";

test("final punctuation character completes the line",()=>{
  const state=new TypingState();
  for(const key of "a;/")state.type(key,key,false);
  assert.equal(state.complete("a;/"),true);
  assert.deepEqual([state.correct,state.errors],[3,0]);
});

test("stop on error holds position and preserves the visible miss",()=>{
  const state=new TypingState();
  assert.equal(state.type("f","j",true),undefined);
  assert.equal(state.pos,0);
  assert.equal(state.marks[0],"incorrect");
  assert.equal(state.type("f","f",true),"corrected");
  assert.deepEqual([state.pos,state.correct,state.errors],[1,1,1]);
});

test("backspace corrects display without erasing historical errors",()=>{
  const state=new TypingState();
  state.type("f","j",false);
  assert.equal(state.backspace(),true);
  state.type("f","f",false);
  assert.deepEqual([state.pos,state.correct,state.errors],[1,1,1]);
});

test("pause time is excluded from elapsed time",()=>{
  assert.equal(activeElapsed(9000,1000,5000,0),4000);
  const pausedTotal=resumeElapsed(8000,5000,1000);
  assert.equal(pausedTotal,4000);
  assert.equal(activeElapsed(10000,1000,0,pausedTotal),5000);
});
