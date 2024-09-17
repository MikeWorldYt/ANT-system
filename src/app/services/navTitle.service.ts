import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class TitleService {
  private titleValue = new BehaviorSubject<string>('');
  currentTitle$ = this.titleValue.asObservable();

  setTitle(section: string) {
    this.titleValue.next(section)
  }

  getTitle(): string {
    return this.titleValue.value;
  }
}