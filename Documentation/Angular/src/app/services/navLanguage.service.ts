import { Injectable } from '@angular/core';
import { BehaviorSubject } from 'rxjs';

@Injectable({
  providedIn: 'root'
})

export class LanguageService {
  private languageValue = new BehaviorSubject<string>('EN');
  currentLanguage$ = this.languageValue.asObservable();
  
  setLanguage(language: string) {
    this.languageValue.next(language);
  }

  getLanguage(): string {
    return this.languageValue.getValue();
  }
}