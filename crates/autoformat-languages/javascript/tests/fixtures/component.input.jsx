import React,{useState} from "react";
import * as path from 'path';

// A small component.
export default function Counter({initial=0,label}){
  const [count,setCount]=useState(initial);
  const double=(n)=>n*2;
  function* ids(){ let i=0; while(true) yield i++; }
  if(count>10){
    console.log(`big ${count}`)
  }else if(count<0) return null
  else{
    setCount(c=>c+1)
  }
  const obj={a:1,'b':[1,2,3],nested:{deep:true},fn(){return this.a}};
  const re=/[a-z]+\/x/g;
  return (
      <div className="counter" onClick={() => setCount(count + 1)}>
        <span>{label}: {count}</span>
        {count > 5 && <b>High</b>}
      </div>
  );
}

switch(action.type){
  case 'add':
    return state+1;
  case 'remove': {
    return state-1;
  }
  default:
    return state;
}
for(let i=0;i<10;i++){ total+=i; }
try{ risky(); }catch(error){ handle(error); }finally{ done(); }
const longCall = someFunction(firstArgumentValue, secondArgumentValue, thirdArgumentValue, fourthArgumentValue, fifth);
const value = condition ? someValue : otherValue;
const chained = promise.then(result => result.json()).then(data => process(data)).catch(err => console.error(err));
