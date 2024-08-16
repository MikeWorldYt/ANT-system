import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class TitleStateService {
  private currTitleSubject = new BehaviorSubject<string>('introduction');
  currTitle$ = this.currTitleSubject.asObservable();

  setCurrTitle(section: string) {
    this.currTitleSubject.next(section);
  }

  getCurrTitle(): string {
    return this.currTitleSubject.value;
  }
}