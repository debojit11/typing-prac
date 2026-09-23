export type Mark="correct"|"incorrect"|"corrected"|undefined;

export class TypingState {
  pos=0;
  correct=0;
  errors=0;
  marks:Mark[]=[];
  blockedError=false;

  resetSession(){this.pos=this.correct=this.errors=0;this.marks=[];this.blockedError=false}
  resetLine(){this.pos=0;this.marks=[];this.blockedError=false}

  type(expected:string,actual:string,stopOnError:boolean):Mark {
    if(actual!==expected){
      this.errors++;
      this.marks[this.pos]="incorrect";
      if(stopOnError){this.blockedError=true;return undefined}
      this.pos++;
      return "incorrect";
    }
    this.correct++;
    const mark=this.blockedError?"corrected":"correct";
    this.marks[this.pos]=mark;
    this.blockedError=false;
    this.pos++;
    return mark;
  }

  backspace():boolean {
    if(this.blockedError){this.marks[this.pos]=undefined;this.blockedError=false;return true}
    if(this.pos===0)return false;
    this.pos--;
    const removed=this.marks[this.pos];
    if(removed==="correct"||removed==="corrected")this.correct--;
    this.marks[this.pos]=undefined;
    return true;
  }

  complete(line:string){return this.pos>=line.length}
}

export function activeElapsed(now:number,startedAt:number,pausedAt:number,pausedTotal:number){
  return Math.max(0,(pausedAt||now)-startedAt-pausedTotal);
}

export function resumeElapsed(now:number,pausedAt:number,pausedTotal:number){
  return pausedAt?pausedTotal+now-pausedAt:pausedTotal;
}
