import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

@Injectable({
  providedIn: 'root'
})
export class TitleStateService {
  private currentSectionSubject = new BehaviorSubject<string>('introduction');
  currentSection$ = this.currentSectionSubject.asObservable();

  setCurrentSection(section: string) {
    this.currentSectionSubject.next(section);
  }

  getCurrentSection(): string {
    return this.currentSectionSubject.value;
  }
}